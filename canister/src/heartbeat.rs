use crate::{
    runtime::{call_get_successors, print},
    state::{self, ResponseToProcess},
    types::{
        GetSuccessorsCompleteResponse, GetSuccessorsRequest, GetSuccessorsRequestInitial,
        GetSuccessorsResponse,
    },
};
use crate::{with_state, with_state_mut};
use bitcoin::consensus::Decodable;
use bitcoin::Block as BitcoinBlock;
use ic_btc_interface::Flag;
use ic_btc_types::{Block, BlockHash};

/// The heartbeat of the Bitcoin canister.
///
/// The heartbeat fetches new blocks from the bitcoin network and inserts them into the state.
pub async fn heartbeat() {
    print("Starting heartbeat...");

    cycles_burn();
    print("Done burning received cycles.");

    if ingest_stable_blocks_into_utxoset() {
        // Exit the heartbeat if stable blocks had been ingested.
        // This is a precaution to not exceed the instructions limit.
        print("Done ingesting stable blocks.");
        return;
    }

    if maybe_fetch_blocks().await {
        // Exit the heartbeat if new blocks have been fetched.
        // This is a precaution to not exceed the instructions limit.
        print("Done fetching new response.");
        return;
    }

    maybe_process_response();
}

// Fetches new blocks if there isn't a request in progress and no complete response to process.
// Returns true if a call to the `blocks_source` has been made, false otherwise.
async fn maybe_fetch_blocks() -> bool {
    if with_state(|s| s.syncing_state.syncing == Flag::Disabled) {
        // Syncing is disabled.
        return false;
    }

    // A guard to verify we aren't already fetching blocks.
    let _guard = match crate::guard::FetchBlocksGuard::new() {
        Some(guard) => guard,
        None => return false,
    };

    // Request additional blocks.
    let maybe_request = maybe_get_successors_request();
    let request = match maybe_request {
        Some(request) => request,
        None => {
            // No request to send at this time.
            return false;
        }
    };

    print(&format!("Sending request: {:?}", request));

    let response: Result<(GetSuccessorsResponse,), _> =
        call_get_successors(with_state(|s| s.blocks_source), request).await;

    print(&format!("Received response: {:?}", response));

    // Save the response.
    with_state_mut(|s| {
        let response = match response {
            Ok((response,)) => response,
            Err((code, msg)) => {
                s.syncing_state.num_get_successors_rejects += 1;
                print(&format!("Error fetching blocks: [{:?}] {}", code, msg));
                s.syncing_state.response_to_process = None;
                return;
            }
        };

        match response {
            GetSuccessorsResponse::Complete(response) => {
                // Received complete response.
                assert!(
                    s.syncing_state.response_to_process.is_none(),
                    "Received complete response before processing previous response."
                );
                s.syncing_state.response_to_process = Some(ResponseToProcess::Complete(response));
            }
            GetSuccessorsResponse::Partial(partial_response) => {
                // Received partial response.
                assert!(
                    s.syncing_state.response_to_process.is_none(),
                    "Received partial response before processing previous response."
                );
                s.syncing_state.response_to_process =
                    Some(ResponseToProcess::Partial(partial_response, 0));
            }
            GetSuccessorsResponse::FollowUp(mut block_bytes) => {
                // Received a follow-up response.
                // A follow-up response is only expected, and only makes sense, when there's
                // a partial response to process.

                let (mut partial_response, mut follow_up_index) = match s.syncing_state.response_to_process.take() {
                    Some(ResponseToProcess::Partial(res, pages)) => (res, pages),
                    other => unreachable!("Cannot receive follow-up response without a previous partial response. Previous response found: {:?}", other)
                };

                // Append block to partial response and increment # pages processed.
                partial_response.partial_block.append(&mut block_bytes);
                follow_up_index += 1;

                // If the response is now complete, store a complete response to process.
                // Otherwise, store the updated partial response.
                s.syncing_state.response_to_process = Some(
                    if follow_up_index == partial_response.remaining_follow_ups {
                        ResponseToProcess::Complete(GetSuccessorsCompleteResponse {
                            blocks: vec![partial_response.partial_block],
                            next: partial_response.next,
                        })
                    } else {
                        ResponseToProcess::Partial(partial_response, follow_up_index)
                    },
                );
            }
        };
    });

    // A request to fetch new blocks has been made.
    true
}

fn ingest_stable_blocks_into_utxoset() -> bool {
    with_state_mut(state::ingest_stable_blocks_into_utxoset)
}

// Process a `GetSuccessorsResponse` if one is available.
fn maybe_process_response() {
    with_state_mut(|state| {
        let response_to_process = state.syncing_state.response_to_process.take();

        match response_to_process {
            Some(ResponseToProcess::Complete(response)) => {
                print(&format!(
                    "Inserting {} blocks from response...",
                    response.blocks.len()
                ));
                for block_bytes in response.blocks.iter() {
                    // Deserialize the block.
                    let block = match BitcoinBlock::consensus_decode(block_bytes.as_slice()) {
                        Ok(block) => block,
                        Err(err) => {
                            print(&format!(
                                "ERROR: Cannot deserialize block. Err: {:?}, Block bytes: {:?}. Full Response: {:?}",
                                err,
                                block_bytes,
                                response,
                            ));

                            // Return, the remaining blocks in the response are dropped.
                            state.syncing_state.num_block_deserialize_errors += 1;
                            return;
                        }
                    };

                    if let Err(err) = state::insert_block(state, Block::new(block)) {
                        print(&format!(
                            "ERROR: Failed to insert block. Err: {:?}, Block bytes: {:?}",
                            err, block_bytes,
                        ));

                        // Return, the remaining blocks in the response are dropped.
                        state.syncing_state.num_insert_block_errors += 1;
                        return;
                    }
                }

                print(&format!(
                    "Inserting {} next block headers...",
                    response.next.len()
                ));
                state::insert_next_block_headers(state, &response.next);
            }
            other => {
                if other.is_some() {
                    print(&format!(
                        "Complete response not yet available. Response so far: {:?}",
                        other
                    ));
                } else {
                    print("No response available to process.");
                }

                // Not a complete response. Put it back into the state.
                state.syncing_state.response_to_process = other;
            }
        }
    });
}

// Retrieves a `GetSuccessorsRequest` to send to the adapter.
fn maybe_get_successors_request() -> Option<GetSuccessorsRequest> {
    with_state(|state| match &state.syncing_state.response_to_process {
        Some(ResponseToProcess::Complete(_)) => {
            // There's already a complete response waiting to be processed.
            None
        }
        Some(ResponseToProcess::Partial(partial_response, follow_up_index)) => {
            // There's a partial response. Create a follow-up request.
            assert!(partial_response.remaining_follow_ups >= *follow_up_index);
            Some(GetSuccessorsRequest::FollowUp(*follow_up_index))
        }
        None => {
            // No response is present. Send an initial request for new blocks.
            let mut processed_block_hashes: Vec<BlockHash> = state::get_unstable_blocks(state)
                .iter()
                .map(|b| b.block_hash())
                .collect();

            // We are guaranteed that there's always at least one block.
            let anchor = processed_block_hashes.remove(0);

            Some(GetSuccessorsRequest::Initial(GetSuccessorsRequestInitial {
                network: state.network(),
                anchor,
                processed_block_hashes,
            }))
        }
    })
}

#[cfg(target_arch = "wasm32")]
fn cycles_burn() {
    let cycles_burnt = ic_cdk::api::cycles_burn(ic_cdk::api::canister_balance128());
    with_state_mut(|s| {
        if let Some(metric_cycles_burnt) = &mut s.metrics.cycles_burnt {
            *metric_cycles_burnt += cycles_burnt;
        } else {
            s.metrics.cycles_burnt = Some(cycles_burnt);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn cycles_burn() {
    let cycles_burnt = 1_000_000;
    with_state_mut(|s| {
        if let Some(metric_cycles_burnt) = &mut s.metrics.cycles_burnt {
            *metric_cycles_burnt += cycles_burnt;
        } else {
            s.metrics.cycles_burnt = Some(cycles_burnt);
        }
    });
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        genesis_block, init,
        runtime::{self, GetSuccessorsReply},
        test_utils::{random_p2pkh_address, BlockBuilder, BlockChainBuilder, TransactionBuilder},
        types::{Address, BlockBlob, GetSuccessorsCompleteResponse, GetSuccessorsPartialResponse},
        utxo_set::IngestingBlock,
    };
    use bitcoin::BlockHeader;
    use ic_btc_interface::{Config, Network};

    fn build_block(prev_header: &BlockHeader, address: Address, num_transactions: u128) -> Block {
        let mut block = BlockBuilder::with_prev_header(prev_header);
        let mut value = 1;
        for _ in 0..num_transactions {
            block = block.with_transaction(
                TransactionBuilder::coinbase()
                    .with_output(&address, value)
                    .build(),
            );

            // Increment the value so that all transaction IDs are unique.
            value += 1;
        }

        block.build()
    }

    #[async_std::test]
    async fn fetches_blocks_and_processes_response() {
        let network = Network::Regtest;

        init(Config {
            stability_threshold: 0,
            network,
            ..Default::default()
        });

        let block = BlockBuilder::with_prev_header(genesis_block(network).header()).build();

        let mut block_bytes = vec![];
        block.consensus_encode(&mut block_bytes).unwrap();

        runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks: vec![block_bytes],
                next: vec![],
            },
        )));

        // Fetch blocks.
        heartbeat().await;

        // Process response.
        heartbeat().await;

        // Assert that the block has been ingested.
        assert_eq!(with_state(state::main_chain_height), 1);

        // The UTXO set hasn't been updated with the genesis block yet.
        assert_eq!(with_state(|s| s.utxos.next_height()), 0);

        // Ingest the stable block (the genesis block) to the UTXO set.
        heartbeat().await;

        // Assert that the block has been ingested.
        assert_eq!(with_state(state::main_chain_height), 1);

        // The UTXO set has been updated with the genesis block.
        assert_eq!(with_state(|s| s.utxos.next_height()), 1);
    }

    #[async_std::test]
    async fn does_not_fetch_blocks_if_syncing_is_disabled() {
        let network = Network::Regtest;

        init(Config {
            stability_threshold: 0,
            network,
            ..Default::default()
        });

        with_state_mut(|s| {
            s.syncing_state.syncing = Flag::Disabled;
        });

        let block = BlockBuilder::with_prev_header(genesis_block(network).header()).build();

        let mut block_bytes = vec![];
        block.consensus_encode(&mut block_bytes).unwrap();

        runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks: vec![block_bytes],
                next: vec![],
            },
        )));

        // Try to fetch blocks
        heartbeat().await;
        heartbeat().await;

        // Assert that the block has not been ingested.
        assert_eq!(with_state(state::main_chain_height), 0);
    }

    #[async_std::test]
    async fn time_slices_large_blocks() {
        let network = Network::Regtest;

        init(Config {
            stability_threshold: 0,
            network,
            ..Default::default()
        });

        // Setup a chain of two blocks.
        let address = random_p2pkh_address(network);
        let block_1 = build_block(genesis_block(network).header(), address.clone(), 6);
        let block_2 = build_block(block_1.header(), address, 1);

        // Serialize the blocks.
        let blocks: Vec<BlockBlob> = [block_1.clone(), block_2]
            .iter()
            .map(|block| {
                let mut block_bytes = vec![];
                block.consensus_encode(&mut block_bytes).unwrap();
                block_bytes
            })
            .collect();

        runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks,
                next: vec![],
            },
        )));

        // Set a large step for the performance_counter to exceed the instructions limit quickly.
        // This value allows ingesting 3 inputs/outputs per round.
        runtime::set_performance_counter_step(1_000_000_000);

        // Fetch blocks.
        heartbeat().await;

        // Process response.
        heartbeat().await;

        // Assert that the blocks have been ingested.
        assert_eq!(with_state(state::main_chain_height), 2);

        // Ingest stable blocks.
        runtime::performance_counter_reset();
        heartbeat().await;

        // Assert that execution has been paused.
        // Ingested the genesis block (1 tx) + 2 txs of block_1 into the UTXO set.
        let partial_block = with_state(|s| s.utxos.ingesting_block.clone().unwrap());
        assert_eq!(partial_block.block, block_1);
        assert_eq!(partial_block.next_tx_idx, 2);
        assert_eq!(partial_block.next_input_idx, 1);
        assert_eq!(partial_block.next_output_idx, 0);

        // Ingest more stable blocks.
        runtime::performance_counter_reset();
        heartbeat().await;

        // Assert that execution has been paused. Ingested 3 more txs in block_1.
        let partial_block = with_state(|s| s.utxos.ingesting_block.clone().unwrap());
        assert_eq!(partial_block.block, block_1);
        assert_eq!(partial_block.next_tx_idx, 5);
        assert_eq!(partial_block.next_input_idx, 1);
        assert_eq!(partial_block.next_output_idx, 0);

        // Only the genesis block has been fully processed, so the stable height is one.
        assert_eq!(with_state(|s| s.utxos.next_height()), 1);

        // Ingest more stable blocks.
        runtime::performance_counter_reset();
        heartbeat().await;

        // Time slicing is complete.
        assert!(with_state(|s| s.utxos.ingesting_block.is_none()));

        // Assert that the blocks have been ingested.
        assert_eq!(with_state(state::main_chain_height), 2);

        // The stable height is now updated to include `block_1`.
        assert_eq!(with_state(|s| s.utxos.next_height()), 2);
    }

    #[async_std::test]
    async fn time_slices_large_transactions() {
        let network = Network::Regtest;
        // The number of inputs/outputs in a transaction.
        let tx_cardinality = 6;

        init(Config {
            stability_threshold: 0,
            network,
            ..Default::default()
        });

        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);

        // Create a transaction where a few inputs are given to address 1.
        let mut tx_1 = TransactionBuilder::coinbase();
        for _ in 0..tx_cardinality {
            tx_1 = tx_1.with_output(&address_1, 1000);
        }
        let tx_1 = tx_1.build();

        // Create another transaction where the UTXOs of address 1 are transferred to address 2.
        let mut tx_2 = TransactionBuilder::new();
        for i in 0..tx_cardinality {
            tx_2 = tx_2.with_input(ic_btc_types::OutPoint {
                txid: tx_1.txid(),
                vout: i,
            });
        }
        for _ in 0..tx_cardinality {
            tx_2 = tx_2.with_output(&address_2, 1000);
        }
        let tx_2 = tx_2.build();

        // Create blocks with the two transactions above.
        let block_1 = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(tx_1)
            .build();

        let block_2 = BlockBuilder::with_prev_header(block_1.header())
            .with_transaction(tx_2)
            .build();

        // An additional block so that the previous blocks are ingested into the stable UTXO set.
        let block_3 = BlockBuilder::with_prev_header(block_2.header()).build();

        // Serialize the blocks.
        let blocks: Vec<BlockBlob> = [block_1.clone(), block_2.clone(), block_3]
            .iter()
            .map(|block| {
                let mut block_bytes = vec![];
                block.consensus_encode(&mut block_bytes).unwrap();
                block_bytes
            })
            .collect();

        runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks,
                next: vec![],
            },
        )));

        // Set a large step for the performance_counter to exceed the instructions limit quickly.
        // This value allows ingesting 3 transactions inputs/outputs per round.
        runtime::set_performance_counter_step(1_000_000_000);

        // Fetch blocks.
        heartbeat().await;

        // Process response.
        heartbeat().await;

        // Assert that the blocks have been ingested.
        assert_eq!(with_state(state::main_chain_height), 3);

        // Run the heartbeat a few rounds to ingest the two stable blocks.
        // Three inputs/outputs are expected to be ingested per round.
        let expected_states = vec![
            IngestingBlock::new_with_args(block_1.clone(), 0, 1, 2),
            IngestingBlock::new_with_args(block_1.clone(), 0, 1, 5),
            IngestingBlock::new_with_args(block_2.clone(), 0, 2, 0),
            IngestingBlock::new_with_args(block_2.clone(), 0, 5, 0),
            IngestingBlock::new_with_args(block_2.clone(), 0, 6, 2),
            IngestingBlock::new_with_args(block_2.clone(), 0, 6, 5),
        ];

        for expected_state in expected_states.into_iter() {
            // Ingest stable blocks.
            runtime::performance_counter_reset();
            heartbeat().await;

            // Assert that execution has been paused.
            let partial_block = with_state(|s| s.utxos.ingesting_block.clone().unwrap());
            assert_eq!(partial_block.block, expected_state.block);
            assert_eq!(partial_block.next_tx_idx, expected_state.next_tx_idx);
            assert_eq!(partial_block.next_input_idx, expected_state.next_input_idx);
            assert_eq!(
                partial_block.next_output_idx,
                expected_state.next_output_idx
            );

            // The addresses 1 and 2 do not change while ingestion is in progress.
            assert_eq!(
                crate::api::get_balance(crate::types::GetBalanceRequest {
                    address: address_1.to_string(),
                    min_confirmations: None
                })
                .unwrap(),
                0
            );

            assert_eq!(
                crate::api::get_balance(crate::types::GetBalanceRequest {
                    address: address_2.to_string(),
                    min_confirmations: None
                })
                .unwrap(),
                tx_cardinality as u64 * 1000
            );
        }

        // Assert ingestion has finished.
        runtime::performance_counter_reset();
        heartbeat().await;
        with_state(|s| assert_eq!(s.utxos.ingesting_block, None));

        // Assert that the blocks have been ingested.
        assert_eq!(with_state(state::main_chain_height), 3);

        // The stable height is now updated to include `block_1` and `block_2`.
        assert_eq!(with_state(|s| s.utxos.next_height()), 3);

        // Query the balance, expecting address 1 to be empty and address 2 to be non-empty.
        assert_eq!(
            crate::api::get_balance(crate::types::GetBalanceRequest {
                address: address_1.to_string(),
                min_confirmations: None
            })
            .unwrap(),
            0
        );

        assert_eq!(
            crate::api::get_balance(crate::types::GetBalanceRequest {
                address: address_2.to_string(),
                min_confirmations: None
            })
            .unwrap(),
            tx_cardinality as u64 * 1000
        );
    }

    #[async_std::test]
    async fn fetches_and_processes_responses_paginated() {
        let network = Network::Regtest;

        init(Config {
            stability_threshold: 0,
            network,
            ..Default::default()
        });

        let address = random_p2pkh_address(network);
        let block = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(
                TransactionBuilder::coinbase()
                    .with_output(&address, 1000)
                    .build(),
            )
            .build();

        let mut block_bytes = vec![];
        block.consensus_encode(&mut block_bytes).unwrap();

        // Split the block bytes into three pages.
        runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Partial(
            GetSuccessorsPartialResponse {
                partial_block: block_bytes[0..40].to_vec(),
                next: vec![],
                remaining_follow_ups: 2,
            },
        )));

        // Fetch blocks (initial response).
        heartbeat().await;

        // Fetch blocks (second page).
        runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::FollowUp(
            block_bytes[40..80].to_vec(),
        )));
        heartbeat().await;

        // Fetch blocks (third page).
        runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::FollowUp(
            block_bytes[80..].to_vec(),
        )));
        heartbeat().await;

        // The response hasn't been fully processed yet, so the balance should still be zero.
        assert_eq!(
            crate::api::get_balance(crate::types::GetBalanceRequest {
                address: address.to_string(),
                min_confirmations: None
            })
            .unwrap(),
            0
        );

        // Process response.
        heartbeat().await;

        // Query the balance, validating the block was processed.
        assert_eq!(
            crate::api::get_balance(crate::types::GetBalanceRequest {
                address: address.to_string(),
                min_confirmations: None
            })
            .unwrap(),
            1000
        );
    }

    #[async_std::test]
    async fn handles_block_deserialize_errors() {
        init(Config::default());

        runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks: vec![
                    // Invalid block.
                    vec![1, 2, 3],
                ],
                next: vec![],
            },
        )));

        // Fetch response.
        heartbeat().await;

        // The number of deserialize errors is still zero.
        assert_eq!(
            with_state(|s| s.syncing_state.num_block_deserialize_errors),
            0
        );

        // Process response.
        heartbeat().await;

        // The number of deserialize errors has been incremented to one and response is dropped.
        with_state(|s| {
            assert_eq!(s.syncing_state.num_block_deserialize_errors, 1);
            assert_eq!(s.syncing_state.response_to_process, None);
        });
    }

    #[async_std::test]
    async fn handles_blocks_that_dont_extend_tree() {
        init(Config::default());

        let mut block_bytes = vec![];
        genesis_block(Network::Regtest)
            .consensus_encode(&mut block_bytes)
            .unwrap();

        runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks: vec![
                    // A valid block, but doesn't extend the tree.
                    block_bytes,
                ],
                next: vec![],
            },
        )));

        // Fetch response.
        heartbeat().await;

        // The number of insert block errors is still zero.
        assert_eq!(with_state(|s| s.syncing_state.num_insert_block_errors), 0);

        // Process response.
        heartbeat().await;

        // The number of insert block errors has been incremented to one and response is dropped.
        with_state(|s| {
            assert_eq!(s.syncing_state.num_insert_block_errors, 1);
            assert_eq!(s.syncing_state.response_to_process, None);
        });
    }

    #[async_std::test]
    async fn block_headers_are_not_inserted_above_instructions_threshold() {
        let network = Network::Regtest;

        init(Config {
            network,
            ..Default::default()
        });

        let next_block_headers = BlockChainBuilder::new(50)
            .build()
            .into_iter()
            .skip(1)
            .map(|b| b.header().into())
            .collect();

        runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks: vec![],
                next: next_block_headers,
            },
        )));

        // Set a large step for the performance_counter to exceed the instructions limit quickly.
        runtime::set_performance_counter_step(1_000_000_000);

        // Fetch blocks.
        heartbeat().await;

        // Process response.
        heartbeat().await;

        // Even though there were 50 next block headers, only 30 were processed due to reaching
        // the instruction limit threshold.
        assert_eq!(
            with_state(|s| s.unstable_blocks.next_block_headers_max_height()),
            Some(30)
        );

        // Run heartbeat again.
        heartbeat().await;
        // There were no more block headers processed.
        assert_eq!(
            with_state(|s| s.unstable_blocks.next_block_headers_max_height()),
            Some(30)
        );
    }
}
