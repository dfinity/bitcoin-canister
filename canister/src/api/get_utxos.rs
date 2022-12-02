use crate::{
    blocktree::BlockChain,
    charge_cycles,
    runtime::{performance_counter, print},
    types::{Address, GetUtxosRequest, OutPoint, Page, Txid, Utxo},
    unstable_blocks, verify_has_enough_cycles, with_state, with_state_mut, State,
};
use ic_btc_types::{GetUtxosError, GetUtxosResponse, Utxo as PublicUtxo, UtxosFilter};
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

/// Retrieves the UTXOs of the given Bitcoin address.
pub fn get_utxos(request: GetUtxosRequest) -> GetUtxosResponse {
    verify_has_enough_cycles(with_state(|s| s.fees.get_utxos_maximum));

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
    })
    .expect("get_utxos failed");

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

    // Charge the fee.
    with_state(|s| {
        let fee = std::cmp::min(
            s.fees.get_utxos_base
                + (stats.ins_total / 10) as u128 * s.fees.get_utxos_cycles_per_ten_instructions,
            s.fees.get_utxos_maximum,
        );
        charge_cycles(fee);
    });

    // Print the number of instructions it took to process this request.
    print(&format!("[INSTRUCTION COUNT] {:?}: {:?}", request, stats));
    res
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
    let chain_height = state.utxos.next_height() + (chain.len() as u32) - 1;

    let mut tip_block_hash = chain.first().block_hash();
    let mut tip_block_height = state.utxos.next_height();

    // Apply unstable blocks to the UTXO set.
    let ins_start = performance_counter();
    for (i, block) in chain.into_chain().iter().enumerate() {
        let block_height = state.utxos.next_height() + (i as u32);
        let confirmations = chain_height - block_height + 1;

        if confirmations < min_confirmations {
            // The block has fewer confirmations than requested.
            // We can stop now since all remaining blocks will have fewer confirmations.
            break;
        }

        address_utxos.apply_block(block);

        tip_block_hash = block.block_hash();
        tip_block_height = block_height;
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
                outpoint: ic_btc_types::OutPoint {
                    vout: utxo.outpoint.vout,
                    txid: utxo.outpoint.txid.to_vec(),
                },
            }
        })
        .collect();

    // If there are remaining UTXOs, then add the pagination offset to the response.
    let rest = utxos.split_off(utxos.len().min(utxo_limit as usize));
    let next_page = rest.first().map(|next| {
        Page {
            tip_block_hash: tip_block_hash.clone(),
            height: next.height,
            outpoint: OutPoint::new(Txid::from(next.outpoint.txid.clone()), next.outpoint.vout),
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
        test_utils::{random_p2pkh_address, random_p2tr_address, BlockBuilder, TransactionBuilder},
        types::{Block, Config, Fees, Network},
        with_state_mut,
    };
    use ic_btc_types::{OutPoint, Utxo};
    use proptest::prelude::*;

    #[test]
    #[should_panic(expected = "get_utxos failed: MalformedAddress")]
    fn get_utxos_malformed_address() {
        crate::init(Config {
            stability_threshold: 1,
            network: Network::Mainnet,
            ..Default::default()
        });

        get_utxos(GetUtxosRequest {
            address: String::from("not an address"),
            filter: None,
        });
    }

    #[test]
    fn genesis_block_only() {
        let network = Network::Regtest;
        crate::init(Config {
            stability_threshold: 1,
            network,
            ..Default::default()
        });

        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: random_p2pkh_address(network).to_string(),
                filter: None
            }),
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
        crate::init(Config {
            stability_threshold: 1,
            network,
            ..Default::default()
        });

        // Generate an address.
        let address = random_p2pkh_address(network);

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
            }),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: coinbase_tx.txid().to_vec(),
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

        crate::init(Config {
            stability_threshold: 1,
            network,
            ..Default::default()
        });

        // Generate addresses.
        let address_1 = random_p2tr_address(network);
        let address_2 = random_p2pkh_address(network);

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
                    txid: transactions[i as usize].txid().to_vec(),
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
            }),
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
            }),
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

        crate::init(Config {
            stability_threshold: 1,
            network,
            ..Default::default()
        });

        let address = random_p2tr_address(network);

        // Create a genesis block where 1000 satoshis are given to a taproot address.
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

        // Assert that the UTXOs of the taproot address can be retrieved.
        assert_eq!(
            get_utxos(GetUtxosRequest {
                address: address.to_string(),
                filter: None
            }),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: coinbase_tx.txid().to_vec(),
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

        crate::init(Config {
            stability_threshold: 2,
            network,
            ..Default::default()
        });

        // Generate addresses.
        let address_1 = random_p2pkh_address(network);

        let address_2 = random_p2pkh_address(network);

        // Create a block where 1000 satoshis are given to the address_1, followed
        // by a block where address_1 gives 1000 satoshis to address_2.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        let block_0 = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(coinbase_tx.clone())
            .build();
        let tx = TransactionBuilder::new()
            .with_input(crate::types::OutPoint::new(coinbase_tx.txid(), 0))
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
                }),
                GetUtxosResponse {
                    utxos: vec![Utxo {
                        outpoint: OutPoint {
                            txid: tx.txid().to_vec(),
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
                }),
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
            }),
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
            }),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: coinbase_tx.txid().to_vec(),
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
    #[should_panic(expected = "get_utxos failed: MinConfirmationsTooLarge { given: 2, max: 1 }")]
    fn panics_on_very_large_confirmations() {
        let network = Network::Regtest;
        crate::init(Config {
            stability_threshold: 2,
            network,
            ..Default::default()
        });

        let address = random_p2pkh_address(network);

        for filter in [
            None,
            Some(UtxosFilter::MinConfirmations(1)),
            Some(UtxosFilter::MinConfirmations(2)),
        ] {
            assert_eq!(
                get_utxos(GetUtxosRequest {
                    address: address.to_string(),
                    filter
                }),
                GetUtxosResponse {
                    utxos: vec![],
                    tip_block_hash: genesis_block(network).block_hash().to_vec(),
                    tip_height: 0,
                    next_page: None,
                }
            );
        }

        // The chain only contains the genesis block, so a min_confirmations of 2
        // should panic, as there aren't that many blocks in the chain.
        get_utxos(GetUtxosRequest {
            address: address.to_string(),
            filter: Some(UtxosFilter::MinConfirmations(2)),
        });
    }

    #[test]
    fn utxos_forks() {
        let network = Network::Regtest;

        // Create some BTC addresses.
        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);
        let address_3 = random_p2pkh_address(network);
        let address_4 = random_p2pkh_address(network);

        // Create a genesis block where 1000 satoshis are given to address 1.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();

        let block_0 = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(coinbase_tx.clone())
            .build();

        crate::init(Config {
            stability_threshold: 2,
            network: Network::Regtest,
            ..Default::default()
        });

        with_state_mut(|state| {
            state::insert_block(state, block_0.clone()).unwrap();
        });

        let block_0_utxos = GetUtxosResponse {
            utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: coinbase_tx.txid().to_vec(),
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
            }),
            block_0_utxos
        );

        // Extend block 0 with block 1 that spends the 1000 satoshis and gives them to address 2.
        let tx = TransactionBuilder::new()
            .with_input(crate::types::OutPoint::new(coinbase_tx.txid(), 0))
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
            }),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: tx.txid().to_vec(),
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
            }),
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
            .with_input(crate::types::OutPoint::new(coinbase_tx.txid(), 0))
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
            }),
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
            }),
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
            }),
            block_0_utxos
        );

        // Now extend block 1' with another block that transfers the funds to address 4.
        // In this case, the fork of [block 1', block 2'] will be considered the "main"
        // chain, and will be part of the UTXOs.
        let tx = TransactionBuilder::new()
            .with_input(crate::types::OutPoint::new(tx.txid(), 0))
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
            }),
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
            }),
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
            }),
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
            }),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: tx.txid().to_vec(),
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
    #[should_panic(expected = "get_utxos failed: MinConfirmationsTooLarge { given: 3, max: 2 }")]
    fn get_utxos_min_confirmations_greater_than_chain_height() {
        let network = Network::Regtest;
        let address_1 = random_p2pkh_address(network);

        // Create a block where 1000 satoshis are given to the address_1.
        let tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        let block_0 = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(tx.clone())
            .build();

        crate::init(Config {
            stability_threshold: 1,
            network,
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
            }),
            GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: tx.txid().to_vec(),
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
            }),
            GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: genesis_block(network).block_hash().to_vec(),
                tip_height: 0,
                next_page: None,
            }
        );

        // min confirmations is too large. Should panic.
        get_utxos(GetUtxosRequest {
            address: address_1.to_string(),
            filter: Some(UtxosFilter::MinConfirmations(3)),
        });
    }

    #[test]
    fn get_utxos_does_not_include_other_addresses() {
        for network in [Network::Mainnet, Network::Testnet, Network::Regtest].iter() {
            // Generate addresses.
            let address_1 = random_p2pkh_address(*network);

            let address_2 = random_p2pkh_address(*network);

            // Create a genesis block where 1000 satoshis are given to the address_1, followed
            // by a block where address_1 gives 1000 satoshis to address_2.
            let coinbase_tx = TransactionBuilder::coinbase()
                .with_output(&address_1, 1000)
                .build();
            let block_0 = BlockBuilder::genesis()
                .with_transaction(coinbase_tx.clone())
                .build();
            let tx = TransactionBuilder::new()
                .with_input(crate::types::OutPoint::new(coinbase_tx.txid(), 0))
                .with_output(&address_2, 1000)
                .build();
            let block_1 = BlockBuilder::with_prev_header(block_0.header())
                .with_transaction(tx.clone())
                .build();

            let mut state = State::new(2, *network, block_0);
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
    }

    #[test]
    fn get_utxos_for_address_with_many_of_them_respects_utxo_limit() {
        for network in [Network::Mainnet, Network::Testnet, Network::Regtest].iter() {
            // Generate an address.
            let address = random_p2pkh_address(*network);

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
            let state = State::new(2, *network, block_0.clone());
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
            network in prop_oneof![
                Just(Network::Mainnet),
                Just(Network::Testnet),
                Just(Network::Regtest),
            ],
            num_transactions in 1..20u64,
            num_blocks in 1..10u64,
            utxo_limit in prop_oneof![
                Just(10),
                Just(20),
                Just(50),
                Just(100),
            ],
        ) {
            // Generate an address.
            let address = random_p2pkh_address(network);

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
        crate::init(Config {
            fees: Fees {
                get_utxos_base: 10,
                get_utxos_maximum: 100,
                ..Default::default()
            },
            ..Default::default()
        });

        get_utxos(GetUtxosRequest {
            address: random_p2pkh_address(Network::Regtest).to_string(),
            filter: None,
        });

        assert_eq!(runtime::get_cycles_balance(), 10);
    }

    #[test]
    fn charges_cycles_capped_at_maximum() {
        crate::init(Config {
            fees: Fees {
                get_utxos_base: 10,
                get_utxos_cycles_per_ten_instructions: 10,
                get_utxos_maximum: 100,
                ..Default::default()
            },
            ..Default::default()
        });

        runtime::set_performance_counter_step(1000);
        runtime::inc_performance_counter();

        get_utxos(GetUtxosRequest {
            address: random_p2pkh_address(Network::Regtest).to_string(),
            filter: None,
        });

        // Charging is capped to the maximum fee.
        assert_eq!(runtime::get_cycles_balance(), 100);
    }

    #[test]
    fn charges_cycles_per_instructions() {
        crate::init(Config {
            fees: Fees {
                get_utxos_base: 10,
                get_utxos_cycles_per_ten_instructions: 10,
                get_utxos_maximum: 100_000,
                ..Default::default()
            },
            ..Default::default()
        });

        // Set the number of instructions consumed.
        runtime::set_performance_counter_step(1000);
        runtime::inc_performance_counter();

        get_utxos(GetUtxosRequest {
            address: random_p2pkh_address(Network::Regtest).to_string(),
            filter: None,
        });

        // Base fee + instructions are charged for.
        assert_eq!(runtime::get_cycles_balance(), 10 + 1000);
    }
}
