use crate::{
    blocktree::BlockChain,
    charge_cycles,
    runtime::{performance_counter, print},
    types::{Address, GetUtxosRequest, Page, Utxo},
    unstable_blocks, verify_has_enough_cycles, with_state, with_state_mut, State,
};
use ic_btc_interface::{GetUtxosError, GetUtxosResponse, Utxo as PublicUtxo, UtxosFilter};
use ic_btc_types::{Block, BlockHash, OutPoint, Txid};
use serde_bytes::ByteBuf;
use std::str::FromStr;

// The maximum number of UTXOs that are allowed to be included in a single
// `GetUtxosResponse`.
//
// Given the size of a `Utxo` is 48 bytes, this means that the size of a single
// response can be ~50KiB (considering the size of remaining fields and potential
// overhead for the candid serialization). This is still quite below
// the max response payload size of 2MiB that the IC needs to respect.
//
// The value also conforms to the interface spec which requires that no more
// than 10_000 `Utxo`s are returned in a single response.
const MAX_UTXOS_PER_RESPONSE: usize = 1_000;

// Various profiling stats for tracking the performance of `get_utxos`.
#[derive(Default, Debug)]
struct Stats {
    // The total number of instructions used to process the request.
    ins_total: u64,

    // The number of instructions used to apply the unstable blocks.
    ins_apply_unstable_blocks: u64,

    // The number of instructions used to build the utxos vec.
    ins_build_utxos_vec: u64,
}

fn get_utxos_private(
    request: GetUtxosRequest,
    charge_fees: bool,
) -> Result<GetUtxosResponse, GetUtxosError> {
    if charge_fees {
        verify_has_enough_cycles(with_state(|s| s.fees.get_utxos_maximum));
        // Charge the base fee.
        charge_cycles(with_state(|s| s.fees.get_utxos_base));
    }
    let (res, stats) = with_state(|state| {
        match &request.filter {
            None => {
                // No filter is specified. Return all UTXOs for the address.
                get_utxos_internal(state, &request.address, 0, None, MAX_UTXOS_PER_RESPONSE)
            }
            Some(UtxosFilter::MinConfirmations(min_confirmations)) => {
                // Return UTXOs with the requested number of confirmations.
                get_utxos_internal(
                    state,
                    &request.address,
                    *min_confirmations,
                    None,
                    MAX_UTXOS_PER_RESPONSE,
                )
            }
            Some(UtxosFilter::Page(page)) => get_utxos_internal(
                state,
                &request.address,
                0,
                Some(page.to_vec()),
                MAX_UTXOS_PER_RESPONSE,
            ),
        }
    })?;

    // Observe metrics
    with_state_mut(|s| {
        s.metrics.get_utxos_total.observe(stats.ins_total);
        s.metrics
            .get_utxos_apply_unstable_blocks
            .observe(stats.ins_apply_unstable_blocks);
        s.metrics
            .get_utxos_build_utxos_vec
            .observe(stats.ins_build_utxos_vec);
    });

    // Charge the fee based on the number of the instructions.
    with_state(|s| {
        let fee = std::cmp::min(
            (stats.ins_total / 10) as u128 * s.fees.get_utxos_cycles_per_ten_instructions,
            s.fees.get_utxos_maximum - s.fees.get_utxos_base,
        );
        if charge_fees {
            charge_cycles(fee);
        }
    });

    // Print the number of instructions it took to process this request.
    print(&format!("[INSTRUCTION COUNT] {:?}: {:?}", request, stats));
    Ok(res)
}

/// Retrieves the UTXOs of the given Bitcoin address.
pub fn get_utxos(request: GetUtxosRequest) -> Result<GetUtxosResponse, GetUtxosError> {
    get_utxos_private(request, true)
}

/// Retrieves the UTXOs of the given Bitcoin address
/// without charging for the execution, used only for query calls.
pub fn get_utxos_query(request: GetUtxosRequest) -> Result<GetUtxosResponse, GetUtxosError> {
    get_utxos_private(request, false)
}

// Returns the set of UTXOs for a given bitcoin address.
//
// Transactions with confirmations < `min_confirmations` are not considered.
//
// If the optional `page` is set, then it will be used to return the next chunk
// of UTXOs starting from that page reference.
//
// The optional `utxo_limit` restricts the number of UTXOs that can be included
// in the response in case there are too many UTXOs for this address and they
// cannot fit in a single response. A `page` reference will be returned along
// the list of UTXOs in this case that can be used in a subsequent request
// to retrieve the remaining UTXOs.
fn get_utxos_internal(
    state: &State,
    address: &str,
    min_confirmations: u32,
    page: Option<Vec<u8>>,
    utxo_limit: usize,
) -> Result<(GetUtxosResponse, Stats), GetUtxosError> {
    match page {
        // A page was provided in the request, so we should use it as a basis
        // to compute the next chunk of UTXOs to be returned.
        Some(page) => {
            let Page {
                tip_block_hash,
                height,
                outpoint,
            } = Page::from_bytes(page).map_err(|err| GetUtxosError::MalformedPage { err })?;
            let chain =
                unstable_blocks::get_chain_with_tip(&state.unstable_blocks, &tip_block_hash)
                    .ok_or(GetUtxosError::UnknownTipBlockHash {
                        tip_block_hash: tip_block_hash.to_vec(),
                    })?;
            get_utxos_from_chain(
                state,
                address,
                min_confirmations,
                chain,
                Some(Utxo {
                    height,
                    outpoint,
                    value: 0,
                }),
                utxo_limit,
            )
        }
        // No specific page was provided, so we use the main chain for computing UTXOs.
        None => {
            let chain = unstable_blocks::get_main_chain(&state.unstable_blocks);
            get_utxos_from_chain(state, address, min_confirmations, chain, None, utxo_limit)
        }
    }
}

// Returns the stability count of the given `target_block`.
//
// The stability count of a block is defined as the largest 𝜹 so that the block is 𝜹-stable.
// A block b is 𝜹-stable if the following conditions hold:
//   * d(b) ≥ 𝜹
//   * ∀ b’ ∈ B \ {b}, h(b’) = h(b): d(b) - d(b’) ≥ 𝜹
//
// It follows from the above definition that the stability count is:
// ```
//    D(b) := {b' ∈ B \ {b} | h(b') = h(b)}
//    stability_count(b) = d(b) if |D(b)| = 0 and d(b) - max_{b' ∈ D(b)} d(b') otherwise.
// ```
fn get_stability_count(
    blocks_with_depths_on_the_same_height: &[(&Block, u32)],
    target_block: BlockHash,
) -> i32 {
    let mut max_depth_of_the_other_blocks = 0;
    let mut target_block_depth = 0;
    for (block, depth) in blocks_with_depths_on_the_same_height.iter() {
        if block.block_hash() != target_block {
            max_depth_of_the_other_blocks = std::cmp::max(max_depth_of_the_other_blocks, *depth);
        } else {
            target_block_depth = *depth;
        }
    }
    target_block_depth as i32 - max_depth_of_the_other_blocks as i32
}

fn get_utxos_from_chain(
    state: &State,
    address: &str,
    min_confirmations: u32,
    chain: BlockChain,
    offset: Option<Utxo>,
    utxo_limit: usize,
) -> Result<(GetUtxosResponse, Stats), GetUtxosError> {
    let mut stats = Stats::default();

    let address = Address::from_str(address).map_err(|_| GetUtxosError::MalformedAddress)?;

    if chain.len() < min_confirmations as usize {
        return Err(GetUtxosError::MinConfirmationsTooLarge {
            given: min_confirmations,
            max: chain.len() as u32,
        });
    }

    let mut address_utxos = state.get_utxos(address);

    let mut tip_block_hash = chain.first().block_hash();
    let mut tip_block_height = state.utxos.next_height();

    let blocks_with_depths_by_heights = state.unstable_blocks.blocks_with_depths_by_heights();

    // Apply unstable blocks to the UTXO set.
    let ins_start = performance_counter();
    for (i, block) in chain.into_chain().iter().enumerate() {
        if get_stability_count(&blocks_with_depths_by_heights[i], block.block_hash())
            < min_confirmations as i32
        {
            // The block has a lower stability count than requested.
            // We can stop now since all remaining blocks will have a lower stability count.
            break;
        }
        tip_block_hash = block.block_hash();
        tip_block_height = state.utxos.next_height() + (i as u32);
        address_utxos.apply_block(block);
    }
    stats.ins_apply_unstable_blocks = performance_counter() - ins_start;

    let ins_start = performance_counter();

    // Attempt to retrieve UTXOs up to the given limit + 1. The additional UTXO, if it exists,
    // provides information needed for pagination.
    let (utxos_to_take, overflow) = utxo_limit.overflowing_add(1);
    assert!(!overflow, "overflow when computing utxos to take");

    let mut utxos: Vec<_> = address_utxos
        .into_iter(offset)
        .take(utxos_to_take)
        .map(|utxo| {
            // Convert UTXOs to their public representation.
            // The way UTXOs are represented in the response is different from how it's represented
            // internally because the internal representation of UTXOs offers more type-checks.
            PublicUtxo {
                value: utxo.value,
                height: utxo.height,
                outpoint: ic_btc_interface::OutPoint {
                    vout: utxo.outpoint.vout,
                    txid: utxo.outpoint.txid.into(),
                },
            }
        })
        .collect();

    // If there are remaining UTXOs, then add the pagination offset to the response.
    let rest = utxos.split_off(utxos.len().min(utxo_limit));
    let next_page = rest.first().map(|next| {
        Page {
            tip_block_hash: tip_block_hash.clone(),
            height: next.height,
            outpoint: OutPoint::new(Txid::from(next.outpoint.txid), next.outpoint.vout),
        }
        .to_bytes()
    });

    stats.ins_build_utxos_vec = performance_counter() - ins_start;
    stats.ins_total = performance_counter();

    Ok((
        GetUtxosResponse {
            utxos,
            tip_block_hash: tip_block_hash.to_vec(),
            tip_height: tip_block_height,
            next_page: next_page.map(ByteBuf::from),
        },
        stats,
    ))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        genesis_block, runtime, state,
        test_utils::{BlockBuilder, BlockChainBuilder, TransactionBuilder},
        types::into_bitcoin_network,
        with_state_mut,
    };
    use ic_btc_interface::{Fees, InitConfig, Network, OutPoint, Utxo};
    use ic_btc_test_utils::{
        random_p2pkh_address, random_p2tr_address, random_p2wpkh_address, random_p2wsh_address,
    };
    use ic_btc_types::Block;
    use proptest::prelude::*;

    #[test]
    fn get_utxos_malformed_address() {
        crate::init(InitConfig {
            stability_threshold: Some(1),
            network: Some(Network::Mainnet),
            ..Default::default()
        });
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: String::from("not an address"),
                filter: None,
            }),
            Err(GetUtxosError::MalformedAddress)
        );
    }

    #[test]
    fn get_utxos_query_malformed_address() {
        crate::init(InitConfig {
            stability_threshold: Some(1),
            network: Some(Network::Mainnet),
            ..Default::default()
        });
        assert_eq!(
            get_utxos_query(GetUtxosRequest {
                address: String::from("not an address"),
                filter: None,
            }),
            Err(GetUtxosError::MalformedAddress)
        );
    }

    #[test]
    fn genesis_block_only() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);
        crate::init(InitConfig {
            stability_threshold: Some(1),
            network: Some(network),
            ..Default::default()
        });

        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: random_p2pkh_address(btc_network).to_string(),
                filter: None
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: genesis_block(network).block_hash().to_vec(),
                tip_height: 0,
                next_page: None,
            }
        );
    }

    #[test]
    fn single_block() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);
        crate::init(InitConfig {
            stability_threshold: Some(1),
            network: Some(network),
            ..Default::default()
        });

        // Generate an address.
        let address = random_p2pkh_address(btc_network).into();

        // Create a block where 1000 satoshis are given to the address.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address, 1000)
            .build();
        let block = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(coinbase_tx.clone())
            .build();

        // Insert the block.
        with_state_mut(|state| {
            state::insert_block(state, block.clone()).unwrap();
        });

        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address.to_string(),
                filter: None
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: coinbase_tx.txid().into(),
                        vout: 0
                    },
                    value: 1000,
                    height: 1,
                }],
                tip_block_hash: block.block_hash().to_vec(),
                tip_height: 1,
                next_page: None,
            }
        );
    }

    #[test]
    fn returns_results_in_descending_height_order() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);

        crate::init(InitConfig {
            stability_threshold: Some(1),
            network: Some(network),
            ..Default::default()
        });

        // Generate addresses.
        let address_1 = random_p2tr_address(btc_network).into();
        let address_2 = random_p2pkh_address(btc_network).into();

        // Create a blockchain which alternates between giving some BTC to
        // address_1 and address_2 based on whether we're creating an even
        // or an odd height block.
        let num_blocks = 10;
        let mut prev_block: Block = genesis_block(network);
        let mut transactions = vec![];
        let mut blocks = vec![];
        for i in 0..num_blocks {
            let tx = if i % 2 == 0 {
                TransactionBuilder::coinbase()
                    .with_output(&address_1, i + 1)
                    .build()
            } else {
                TransactionBuilder::coinbase()
                    .with_output(&address_2, i + 1)
                    .build()
            };
            transactions.push(tx.clone());
            let block = BlockBuilder::with_prev_header(prev_block.header())
                .with_transaction(tx.clone())
                .build();

            blocks.push(block.clone());
            prev_block = block;
        }

        // Insert the blocks.
        with_state_mut(|state| {
            for block in blocks.iter() {
                state::insert_block(state, block.clone()).unwrap();
            }
        });

        // We expect that address_2 has `Utxo`s on all even heights and
        // address_1 on all odd heights, both in descending order.
        let mut expected_utxos_address_1 = vec![];
        let mut expected_utxos_address_2 = vec![];
        for i in (0..num_blocks).rev() {
            let expected_utxo = Utxo {
                outpoint: OutPoint {
                    txid: transactions[i as usize].txid().into(),
                    vout: 0,
                },
                value: i + 1,
                height: (i + 1) as u32,
            };
            if i % 2 == 0 {
                expected_utxos_address_1.push(expected_utxo)
            } else {
                expected_utxos_address_2.push(expected_utxo);
            }
        }

        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_1.to_string(),
                filter: None
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: expected_utxos_address_1,
                tip_block_hash: blocks.last().unwrap().block_hash().to_vec(),
                tip_height: num_blocks as u32,
                next_page: None,
            }
        );

        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_2.to_string(),
                filter: None
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: expected_utxos_address_2,
                tip_block_hash: blocks.last().unwrap().block_hash().to_vec(),
                tip_height: num_blocks as u32,
                next_page: None,
            }
        );
    }

    #[test]
    fn supports_taproot_addresses() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);
        supports_address(network, random_p2tr_address(btc_network).into());
    }

    #[test]
    fn supports_p2wpkh_addresses() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);
        supports_address(network, random_p2wpkh_address(btc_network).into());
    }

    #[test]
    fn supports_p2wsh_addresses() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);
        supports_address(network, random_p2wsh_address(btc_network).into());
    }

    // Tests that the provided address is supported and its UTXOs can be fetched.
    fn supports_address(network: Network, address: Address) {
        crate::init(InitConfig {
            network: Some(network),
            ..Default::default()
        });

        // Create a genesis block where 1000 satoshis are given to the address.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address, 1000)
            .build();

        let block = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(coinbase_tx.clone())
            .build();

        // Insert the block
        with_state_mut(|state| {
            state::insert_block(state, block.clone()).unwrap();
        });

        // Assert that the UTXOs of the address can be retrieved.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address.to_string(),
                filter: None
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: coinbase_tx.txid().into(),
                        vout: 0,
                    },
                    value: 1000,
                    height: 1,
                }],
                tip_block_hash: block.block_hash().to_vec(),
                tip_height: 1,
                next_page: None,
            }
        );
    }

    #[test]
    fn min_confirmations() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);

        crate::init(InitConfig {
            stability_threshold: Some(2),
            network: Some(network),
            ..Default::default()
        });

        // Generate addresses.
        let address_1 = random_p2pkh_address(btc_network).into();

        let address_2 = random_p2pkh_address(btc_network).into();

        // Create a block where 1000 satoshis are given to the address_1, followed
        // by a block where address_1 gives 1000 satoshis to address_2.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        let block_0 = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(coinbase_tx.clone())
            .build();
        let tx = TransactionBuilder::new()
            .with_input(ic_btc_types::OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_2, 1000)
            .build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .with_transaction(tx.clone())
            .build();

        // Insert the blocks;
        with_state_mut(|state| {
            state::insert_block(state, block_0.clone()).unwrap();
            state::insert_block(state, block_1.clone()).unwrap();
        });

        // With up to one confirmation, expect address 2 to have one UTXO, and
        // address 1 to have no UTXOs.
        for min_confirmations in [None, Some(0), Some(1)].iter() {
            assert_eq!(
                get_utxos(GetUtxosRequest {
                    address: address_2.to_string(),
                    filter: min_confirmations.map(UtxosFilter::MinConfirmations),
                })
                .unwrap(),
                GetUtxosResponse {
                    utxos: vec![Utxo {
                        outpoint: OutPoint {
                            txid: tx.txid().into(),
                            vout: 0,
                        },
                        value: 1000,
                        height: 2,
                    }],
                    tip_block_hash: block_1.block_hash().to_vec(),
                    tip_height: 2,
                    next_page: None,
                }
            );

            assert_eq!(
                get_utxos(GetUtxosRequest {
                    address: address_1.to_string(),
                    filter: min_confirmations.map(UtxosFilter::MinConfirmations),
                })
                .unwrap(),
                GetUtxosResponse {
                    utxos: vec![],
                    tip_block_hash: block_1.block_hash().to_vec(),
                    tip_height: 2,
                    next_page: None,
                }
            );
        }

        // With two confirmations, expect address 2 to have no UTXOs, and address 1 to
        // have one UTXO.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_2.to_string(),
                filter: Some(UtxosFilter::MinConfirmations(2))
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_0.block_hash().to_vec(),
                tip_height: 1,
                next_page: None,
            }
        );
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_1.to_string(),
                filter: Some(UtxosFilter::MinConfirmations(2))
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: coinbase_tx.txid().into(),
                        vout: 0,
                    },
                    value: 1000,
                    height: 1,
                }],
                tip_block_hash: block_0.block_hash().to_vec(),
                tip_height: 1,
                next_page: None,
            }
        );
    }

    #[test]
    fn error_on_very_large_confirmations() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);

        crate::init(InitConfig {
            stability_threshold: Some(2),
            network: Some(network),
            ..Default::default()
        });

        let address: Address = random_p2pkh_address(btc_network).into();

        for filter in [None, Some(UtxosFilter::MinConfirmations(1))] {
            assert_eq!(
                get_utxos(GetUtxosRequest {
                    address: address.to_string(),
                    filter
                })
                .unwrap(),
                GetUtxosResponse {
                    utxos: vec![],
                    tip_block_hash: genesis_block(network).block_hash().to_vec(),
                    tip_height: 0,
                    next_page: None,
                }
            );
        }

        // The chain only contains the genesis block, so a min_confirmations of 2
        // should return an error, as there aren't that many blocks in the chain.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address.to_string(),
                filter: Some(UtxosFilter::MinConfirmations(2)),
            }),
            Err(GetUtxosError::MinConfirmationsTooLarge { given: 2, max: 1 })
        );
    }

    #[test]
    fn utxos_forks() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);

        // Create some BTC addresses.
        let address_1 = random_p2pkh_address(btc_network).into();
        let address_2 = random_p2pkh_address(btc_network).into();
        let address_3 = random_p2pkh_address(btc_network).into();
        let address_4 = random_p2pkh_address(btc_network).into();

        // Create a genesis block where 1000 satoshis are given to address 1.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();

        let block_0 = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(coinbase_tx.clone())
            .build();

        crate::init(InitConfig {
            stability_threshold: Some(2),
            network: Some(Network::Regtest),
            ..Default::default()
        });

        with_state_mut(|state| {
            state::insert_block(state, block_0.clone()).unwrap();
        });

        let block_0_utxos = GetUtxosResponse {
            utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: coinbase_tx.txid().into(),
                    vout: 0,
                },
                value: 1000,
                height: 1,
            }],
            tip_block_hash: block_0.block_hash().to_vec(),
            tip_height: 1,
            next_page: None,
        };

        // Assert that the UTXOs of address 1 are present.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_1.to_string(),
                filter: None,
            })
            .unwrap(),
            block_0_utxos
        );

        // Extend block 0 with block 1 that spends the 1000 satoshis and gives them to address 2.
        let tx = TransactionBuilder::new()
            .with_input(ic_btc_types::OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_2, 1000)
            .build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .with_transaction(tx.clone())
            .build();

        with_state_mut(|state| {
            state::insert_block(state, block_1.clone()).unwrap();
        });

        // address 2 should now have the UTXO while address 1 has no UTXOs.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_2.to_string(),
                filter: None,
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: tx.txid().into(),
                        vout: 0,
                    },
                    value: 1000,
                    height: 2,
                }],
                tip_block_hash: block_1.block_hash().to_vec(),
                tip_height: 2,
                next_page: None,
            }
        );

        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_1.to_string(),
                filter: None,
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_1.block_hash().to_vec(),
                tip_height: 2,
                next_page: None,
            }
        );

        // Extend block 0 (again) with block 1 that spends the 1000 satoshis to address 3
        // This causes a fork.
        let tx = TransactionBuilder::new()
            .with_input(ic_btc_types::OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_3, 1000)
            .build();
        let block_1_prime = BlockBuilder::with_prev_header(block_0.header())
            .with_transaction(tx.clone())
            .build();

        with_state_mut(|state| {
            state::insert_block(state, block_1_prime.clone()).unwrap();
        });

        // Because block 1 and block 1' contest with each other, neither of them are included
        // in the UTXOs. Only the UTXOs of block 0 are returned.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_2.to_string(),
                filter: None,
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_0.block_hash().to_vec(),
                tip_height: 1,
                next_page: None,
            }
        );
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_3.to_string(),
                filter: None,
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_0.block_hash().to_vec(),
                tip_height: 1,
                next_page: None,
            }
        );
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_1.to_string(),
                filter: None,
            })
            .unwrap(),
            block_0_utxos
        );

        // Now extend block 1' with another block that transfers the funds to address 4.
        // In this case, the fork of [block 1', block 2'] will be considered the "main"
        // chain, and will be part of the UTXOs.
        let tx = TransactionBuilder::new()
            .with_input(ic_btc_types::OutPoint::new(tx.txid(), 0))
            .with_output(&address_4, 1000)
            .build();
        let block_2_prime = BlockBuilder::with_prev_header(block_1_prime.header())
            .with_transaction(tx.clone())
            .build();
        with_state_mut(|state| {
            state::insert_block(state, block_2_prime.clone()).unwrap();
        });

        // Address 1 has no UTXOs since they were spent on the main chain.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_1.to_string(),
                filter: None,
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_2_prime.block_hash().to_vec(),
                tip_height: 3,
                next_page: None,
            }
        );
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_2.to_string(),
                filter: None,
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_2_prime.block_hash().to_vec(),
                tip_height: 3,
                next_page: None,
            }
        );
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_3.to_string(),
                filter: None,
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_2_prime.block_hash().to_vec(),
                tip_height: 3,
                next_page: None,
            }
        );
        // The funds are now with address 4.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_4.to_string(),
                filter: None,
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: tx.txid().into(),
                        vout: 0,
                    },
                    value: 1000,
                    height: 3,
                }],
                tip_block_hash: block_2_prime.block_hash().to_vec(),
                tip_height: 3,
                next_page: None,
            }
        );
    }

    #[test]
    fn get_utxos_min_confirmations_greater_than_chain_height() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);

        let address_1 = random_p2pkh_address(btc_network).into();

        // Create a block where 1000 satoshis are given to the address_1.
        let tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        let block_0 = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(tx.clone())
            .build();

        crate::init(InitConfig {
            stability_threshold: Some(1),
            network: Some(network),
            ..Default::default()
        });

        with_state_mut(|state| {
            state::insert_block(state, block_0.clone()).unwrap();
        });

        // Retrieve the UTXOs at 1 confirmation.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_1.to_string(),
                filter: Some(UtxosFilter::MinConfirmations(1)),
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: tx.txid().into(),
                        vout: 0
                    },
                    value: 1000,
                    height: 1,
                }],
                tip_block_hash: block_0.block_hash().to_vec(),
                tip_height: 1,
                next_page: None,
            }
        );

        // No UTXOs at 2 confirmations.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_1.to_string(),
                filter: Some(UtxosFilter::MinConfirmations(2)),
            })
            .unwrap(),
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: genesis_block(network).block_hash().to_vec(),
                tip_height: 0,
                next_page: None,
            }
        );

        // min confirmations is too large. Should return an error.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address_1.to_string(),
                filter: Some(UtxosFilter::MinConfirmations(3)),
            }),
            Err(GetUtxosError::MinConfirmationsTooLarge { given: 3, max: 2 })
        );
    }

    #[test]
    fn get_utxos_does_not_include_other_addresses() {
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);

        // Generate addresses.
        let address_1 = random_p2pkh_address(btc_network).into();

        let address_2 = random_p2pkh_address(btc_network).into();

        // Create a genesis block where 1000 satoshis are given to the address_1, followed
        // by a block where address_1 gives 1000 satoshis to address_2.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        let block_0 = BlockBuilder::genesis()
            .with_transaction(coinbase_tx.clone())
            .build();
        let tx = TransactionBuilder::new()
            .with_input(ic_btc_types::OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_2, 1000)
            .build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .with_transaction(tx)
            .build();

        let mut state = State::new(2, network, block_0);
        state::insert_block(&mut state, block_1.clone()).unwrap();

        // Address 1 should have no UTXOs at zero confirmations.
        assert_eq!(
            get_utxos_internal(
                &state,
                &address_1.to_string(),
                0,
                None,
                MAX_UTXOS_PER_RESPONSE
            )
            .unwrap()
            .0,
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_1.block_hash().to_vec(),
                tip_height: 1,
                next_page: None,
            }
        );
    }

    #[test]
    fn get_utxos_for_address_with_many_of_them_respects_utxo_limit() {
        for network in [Network::Mainnet, Network::Testnet, Network::Regtest] {
            let btc_network = into_bitcoin_network(network);
            // Generate an address.
            let address = random_p2pkh_address(btc_network).into();

            let num_transactions = 10;
            let mut transactions = vec![];
            for i in 0..num_transactions {
                transactions.push(
                    TransactionBuilder::coinbase()
                        .with_output(&address, (i + 1) * 10)
                        .build(),
                );
            }

            let mut block_builder = BlockBuilder::genesis();
            for transaction in transactions.iter() {
                block_builder = block_builder.with_transaction(transaction.clone());
            }
            let block_0 = block_builder.build();
            let state = State::new(2, network, block_0.clone());
            let tip_block_hash = block_0.block_hash();

            let utxo_set = get_utxos_internal(
                &state,
                &address.to_string(),
                0,
                None,
                MAX_UTXOS_PER_RESPONSE,
            )
            .unwrap()
            .0
            .utxos;

            // Only some UTXOs can be included given that we use a utxo limit.
            let response = get_utxos_internal(
                &state,
                &address.to_string(),
                0,
                None,
                // Allow 3 UTXOs to be returned.
                3,
            )
            .unwrap()
            .0;

            assert_eq!(response.utxos.len(), 3);
            assert!(response.utxos.len() < utxo_set.len());
            assert_eq!(response.tip_block_hash, tip_block_hash.clone().to_vec());
            assert_eq!(response.tip_height, 0);
            assert!(response.next_page.is_some());

            // A bigger limit allows more UTXOs to be included in a single response.
            let response = get_utxos_internal(
                &state,
                &address.to_string(),
                0,
                None,
                // Allow 4 UTXOs to be returned.
                4,
            )
            .unwrap()
            .0;

            assert_eq!(response.utxos.len(), 4);
            assert!(response.utxos.len() < utxo_set.len());
            assert_eq!(response.tip_block_hash, tip_block_hash.clone().to_vec());
            assert_eq!(response.tip_height, 0);
            assert!(response.next_page.is_some());

            // A very big limit will result in the same as requesting UTXOs without any limit.
            let response = get_utxos_internal(&state, &address.to_string(), 0, None, 1000)
                .unwrap()
                .0;

            assert_eq!(response.utxos.len(), num_transactions as usize);
            assert_eq!(response.utxos.len(), utxo_set.len());
            assert_eq!(response.tip_block_hash, tip_block_hash.clone().to_vec());
            assert_eq!(response.tip_height, 0);
            assert!(response.next_page.is_none());
        }
    }

    proptest! {
        #[test]
        fn get_utxos_with_pagination_is_consistent_with_no_pagination(
            num_transactions in 1..20u64,
            num_blocks in 1..10u64,
            utxo_limit in prop_oneof![
                Just(10),
                Just(20),
                Just(50),
                Just(100),
            ],
        ) {
            let network = Network::Regtest;
            let btc_network = into_bitcoin_network(network);

            // Generate an address.
            let address = random_p2pkh_address(btc_network).into();

            let mut prev_block: Option<Block> = None;
            let mut value = 1;
            let mut blocks = vec![];
            for block_idx in 0..num_blocks {
                let mut block_builder = match prev_block {
                    Some(b) => BlockBuilder::with_prev_header(b.header()),
                    None => BlockBuilder::genesis(),
                };

                let mut transactions = vec![];
                for _ in 0..(num_transactions + block_idx) {
                    transactions.push(
                        TransactionBuilder::coinbase()
                            .with_output(&address, value)
                            .build()
                    );
                    // Vary the value of the transaction to ensure that
                    // we get unique outpoints in the blockchain.
                    value += 1;
                }

                for transaction in transactions.iter() {
                    block_builder = block_builder.with_transaction(transaction.clone());
                }

                let block = block_builder.build();
                blocks.push(block.clone());
                prev_block = Some(block);
            }

            let mut state = State::new(2, network, blocks[0].clone());
            for block in blocks[1..].iter() {
                state::insert_block(&mut state, block.clone()).unwrap();
            }

            // Get UTXO set without any pagination...
            let utxo_set = get_utxos_internal(&state, &address.to_string(), 0, None, MAX_UTXOS_PER_RESPONSE)
                .unwrap().0
                .utxos;

            // also get UTXO set with pagination until there are no
            // more pages returned...
            let mut utxos_chunked = vec![];
            let mut page = None;
            loop {
                let response = get_utxos_internal(
                    &state,
                    &address.to_string(),
                    0,
                    page,
                    utxo_limit,
                )
                .unwrap().0;
                utxos_chunked.extend(response.utxos);
                if response.next_page.is_none() {
                    break;
                } else {
                    page = response.next_page.map(|x| x.to_vec());
                }
            }

            // and compare the two results.
            assert_eq!(utxo_set, utxos_chunked);
        }
    }

    #[test]
    fn charges_cycles() {
        crate::init(InitConfig {
            fees: Some(Fees {
                get_utxos_base: 10,
                get_utxos_maximum: 100,
                ..Default::default()
            }),
            ..Default::default()
        });

        get_utxos(GetUtxosRequest {
            address: random_p2pkh_address(bitcoin::Network::Regtest).to_string(),
            filter: None,
        })
        .unwrap();

        assert_eq!(runtime::get_cycles_balance(), 10);
    }

    #[test]
    fn charges_cycles_capped_at_maximum() {
        crate::init(InitConfig {
            fees: Some(Fees {
                get_utxos_base: 10,
                get_utxos_cycles_per_ten_instructions: 10,
                get_utxos_maximum: 100,
                ..Default::default()
            }),
            ..Default::default()
        });

        runtime::set_performance_counter_step(1000);
        runtime::inc_performance_counter();

        get_utxos(GetUtxosRequest {
            address: random_p2pkh_address(bitcoin::Network::Regtest).to_string(),
            filter: None,
        })
        .unwrap();

        // Charging is capped to the maximum fee.
        assert_eq!(runtime::get_cycles_balance(), 100);
    }

    #[test]
    fn charges_cycles_per_instructions() {
        crate::init(InitConfig {
            fees: Some(Fees {
                get_utxos_base: 10,
                get_utxos_cycles_per_ten_instructions: 10,
                get_utxos_maximum: 100_000,
                ..Default::default()
            }),
            ..Default::default()
        });

        // Set the number of instructions consumed.
        runtime::set_performance_counter_step(1000);
        runtime::inc_performance_counter();

        get_utxos(GetUtxosRequest {
            address: random_p2pkh_address(bitcoin::Network::Regtest).to_string(),
            filter: None,
        })
        .unwrap();

        // Base fee + instructions are charged for.
        assert_eq!(runtime::get_cycles_balance(), 10 + 1000);
    }

    #[test]
    fn test_get_stability_count_single_block_on_height() {
        let block = BlockBuilder::genesis().build();
        let blocks_with_depths: Vec<(&Block, u32)> = vec![(&block, 1)];
        // Stability count should be 1.
        assert_eq!(
            get_stability_count(&blocks_with_depths, block.block_hash()),
            1
        );
    }

    #[test]
    fn test_get_stability_count_multiple_blocks_on_height() {
        let block1 = BlockBuilder::genesis().build();
        let block2 = BlockBuilder::genesis().build();
        let block3 = BlockBuilder::genesis().build();

        let blocks_with_depths: Vec<(&Block, u32)> = vec![(&block1, 5), (&block2, 7), (&block3, 3)];
        // The stability_count of block1 should be 5 - 7 = -2.
        assert_eq!(
            get_stability_count(&blocks_with_depths, block1.block_hash()),
            -2
        );
        // The stability_count of block2 should be 7 - 5 = 2.
        assert_eq!(
            get_stability_count(&blocks_with_depths, block2.block_hash()),
            2
        );
        // The stability_count of block3 should be 3 - 7 = -4.
        assert_eq!(
            get_stability_count(&blocks_with_depths, block3.block_hash()),
            -4
        );
    }

    // Documents the behavior of `get_utxos` when min_confirmations = 0.
    #[test]
    fn min_confirmations_zero() {
        // Create a chain with two forks of equal length that looks as follows.
        //
        // A -> B -> C -> D -> E -> F
        // |
        //  \-> B'-> C'-> D'-> E'-> F'
        //
        let chain = BlockChainBuilder::new(6).build();
        let fork = BlockChainBuilder::fork(&chain[0], 5).build();

        crate::init(InitConfig::default());

        // Insert the blocks.
        with_state_mut(|state| {
            for block in chain.iter().skip(1) {
                state::insert_block(state, block.clone()).unwrap();
            }

            for block in fork.into_iter() {
                state::insert_block(state, block).unwrap();
            }
        });

        // Because the forks are of equal length, `A`, the root of the fork,
        // is considered the tip at zero confirmations.
        assert_tip_at_confirmations(0, chain[0].block_hash());

        // Extend the first fork by one block.
        let chain_6 = BlockBuilder::with_prev_header(chain[5].header()).build();
        with_state_mut(|state| {
            state::insert_block(state, chain_6.clone()).unwrap();
        });

        // Now the chain looks like this:
        //
        // A -> B -> C -> D -> E -> F -> G
        // |
        //  \-> B'-> C'-> D'-> E'-> F'
        //
        // Now that one fork is longer, the tip of that fork is considered the tip
        // at zero and one confirmations.
        assert_tip_at_confirmations(0, chain_6.block_hash());
        assert_tip_at_confirmations(1, chain_6.block_hash());

        // A is the tip at 2+ confirmations.
        assert_tip_at_confirmations(2, chain[0].block_hash());
    }

    // Asserts that the given block hash is the tip at the given number of confirmations.
    fn assert_tip_at_confirmations(confirmations: u32, expected_tip: BlockHash) {
        // To fetch the tip, we call `get_utxos` using a random address.
        let address = random_p2pkh_address(bitcoin::Network::Regtest).to_string();
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address,
                filter: Some(UtxosFilter::MinConfirmations(confirmations)),
            })
            .unwrap()
            .tip_block_hash,
            expected_tip.to_vec()
        );
    }
}
