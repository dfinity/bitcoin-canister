use crate::{store, types::GetUtxosRequest, with_state_mut};
use ic_btc_types::{GetUtxosError, GetUtxosResponse, UtxosFilter};

// The maximum number of UTXOs that are allowed to be included in a single
// `GetUtxosResponse`.
//
// Given the size of a `Utxo` is 48 bytes, this means that the size of a single
// response can be ~500KiB (considering the size of remaining fields and
// potential overhead for the candid serialization). This is still quite below
// the max response payload size of 2MiB that the IC needs to respect.

// The value also conforms to the interface spec which requires that no more
// than 100_000 `Utxo`s are returned in a single response.
const MAX_UTXOS_PER_RESPONSE: usize = 10_000;

/// Retrieves the UTXOs of the given Bitcoin address.
pub fn get_utxos(request: GetUtxosRequest) -> GetUtxosResponse {
    get_utxos_internal(&request.address, request.filter).expect("get_utxos failed")
}

fn get_utxos_internal(
    address: &str,
    filter: Option<UtxosFilter>,
) -> Result<GetUtxosResponse, GetUtxosError> {
    with_state_mut(|state| {
        match filter {
            None => {
                // No filter is specified. Return all UTXOs for the address.
                store::get_utxos(state, address, 0, None, Some(MAX_UTXOS_PER_RESPONSE))
            }
            Some(UtxosFilter::MinConfirmations(min_confirmations)) => {
                // Return UTXOs with the requested number of confirmations.
                store::get_utxos(
                    state,
                    address,
                    min_confirmations,
                    None,
                    Some(MAX_UTXOS_PER_RESPONSE),
                )
            }
            Some(UtxosFilter::Page(page)) => store::get_utxos(
                state,
                address,
                0,
                Some(page.to_vec()),
                Some(MAX_UTXOS_PER_RESPONSE),
            ),
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        test_utils::random_p2pkh_address,
        types::{InitPayload, Network},
    };
    use bitcoin::{blockdata::constants::genesis_block, Block};
    use ic_btc_test_utils::{random_p2tr_address, BlockBuilder, TransactionBuilder};
    use ic_btc_types::{OutPoint, Utxo};

    #[test]
    #[should_panic(expected = "get_utxos failed: MalformedAddress")]
    fn get_utxos_malformed_address() {
        crate::init(InitPayload {
            stability_threshold: 1,
            network: Network::Mainnet,
            blocks_source: None,
        });

        get_utxos(GetUtxosRequest {
            address: String::from("not an address"),
            filter: None,
        });
    }

    #[test]
    fn single_block() {
        let network = Network::Regtest;
        crate::init(InitPayload {
            stability_threshold: 1,
            network,
            blocks_source: None,
        });

        // Generate an address.
        let address = random_p2pkh_address(network);

        // Create a block where 1000 satoshis are given to the address.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address, 1000)
            .build();
        let block = BlockBuilder::with_prev_header(genesis_block(network.into()).header)
            .with_transaction(coinbase_tx.clone())
            .build();

        // Insert the block.
        with_state_mut(|state| {
            store::insert_block(state, block.clone()).unwrap();
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

        crate::init(InitPayload {
            stability_threshold: 1,
            network,
            blocks_source: None,
        });

        // Generate addresses.
        let address_1 = random_p2tr_address(network.into());
        let address_2 = random_p2pkh_address(network);

        // Create a blockchain which alternates between giving some BTC to
        // address_1 and address_2 based on whether we're creating an even
        // or an odd height block.
        let num_blocks = 10;
        let mut prev_block: Block = genesis_block(network.into());
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
            let block = BlockBuilder::with_prev_header(prev_block.header)
                .with_transaction(tx.clone())
                .build();

            blocks.push(block.clone());
            prev_block = block;
        }

        // Insert the blocks.
        with_state_mut(|state| {
            for block in blocks.iter() {
                store::insert_block(state, block.clone()).unwrap();
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

        crate::init(InitPayload {
            stability_threshold: 1,
            network,
            blocks_source: None,
        });

        let address = random_p2tr_address(network.into());

        // Create a genesis block where 1000 satoshis are given to a taproot address.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address, 1000)
            .build();

        let block = BlockBuilder::with_prev_header(genesis_block(network.into()).header)
            .with_transaction(coinbase_tx.clone())
            .build();

        // Insert the block
        with_state_mut(|state| {
            store::insert_block(state, block.clone()).unwrap();
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

        crate::init(InitPayload {
            stability_threshold: 2,
            network,
            blocks_source: None,
        });

        // Generate addresses.
        let address_1 = random_p2pkh_address(network);

        let address_2 = random_p2pkh_address(network);

        // Create a block where 1000 satoshis are given to the address_1, followed
        // by a block where address_1 gives 1000 satoshis to address_2.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        let block_0 = BlockBuilder::with_prev_header(genesis_block(network.into()).header)
            .with_transaction(coinbase_tx.clone())
            .build();
        let tx = TransactionBuilder::new()
            .with_input(bitcoin::OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_2, 1000)
            .build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header)
            .with_transaction(tx.clone())
            .build();

        // Insert the blocks;
        with_state_mut(|state| {
            store::insert_block(state, block_0.clone()).unwrap();
            store::insert_block(state, block_1.clone()).unwrap();
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
        crate::init(InitPayload {
            stability_threshold: 2,
            network,
            blocks_source: None,
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
                    tip_block_hash: genesis_block(network.into()).block_hash().to_vec(),
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
}
