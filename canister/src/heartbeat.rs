use crate::{
    runtime::{call_get_successors, print},
    state::{self, ResponseToProcess},
    types::{
        Block, BlockHash, GetSuccessorsCompleteResponse, GetSuccessorsRequest,
        GetSuccessorsRequestInitial, GetSuccessorsResponse,
    },
};
use crate::{with_state, with_state_mut};
use bitcoin::consensus::Decodable;
use bitcoin::Block as BitcoinBlock;

/// The heartbeat of the Bitcoin canister.
///
/// The heartbeat fetches new blocks from the bitcoin network and inserts them into the state.
pub async fn heartbeat() {
    if ingest_stable_blocks_into_utxoset() {
        // Exit the heartbeat if stable blocks had been ingested.
        // This is a precaution to not exceed the instructions limit.
        return;
    }

    if maybe_fetch_blocks().await {
        // Exit the heartbeat if new blocks have been fetched.
        // This is a precaution to not exceed the instructions limit.
        return;
    }

    maybe_process_response();
}

// Fetches new blocks if there isn't a request in progress and no complete response to process.
// Returns true if a call to the `blocks_source` has been made, false otherwise.
async fn maybe_fetch_blocks() -> bool {
    if with_state(|s| s.syncing_state.is_fetching_blocks) {
        // Already fetching blocks.
        return false;
    }

    // Request additional blocks.
    let maybe_request = maybe_get_successors_request();
    let request = match maybe_request {
        Some(request) => request,
        None => {
            // No request to send at this time.
            return false;
        }
    };

    // A lock to ensure the heartbeat only sends one request at a time.
    with_state_mut(|s| {
        s.syncing_state.is_fetching_blocks = true;
    });

    print(&format!("Sending request: {:?}", request));

    let response: Result<(GetSuccessorsResponse,), _> =
        call_get_successors(with_state(|s| s.blocks_source), request).await;

    print(&format!("Received response: {:?}", response));

    // Release the heartbeat lock and save the response.
    with_state_mut(|s| {
        s.syncing_state.is_fetching_blocks = false;

        let response = match response {
            Ok((response,)) => response,
            Err((code, msg)) => {
                // TODO(EXC-1232): track these occurrences in a metric.
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
                for block in response.blocks.iter() {
                    // TODO(EXC-1215): Gracefully handle the errors here.
                    let block = BitcoinBlock::consensus_decode(block.as_slice()).unwrap();
                    state::insert_block(state, Block::new(block)).unwrap();
                }
            }
            other => {
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
                .map(|b| b.block_hash().to_vec())
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        genesis_block, init, runtime,
        state::PartialStableBlock,
        test_utils::{random_p2pkh_address, BlockBuilder, TransactionBuilder},
        types::{
            Address, BlockBlob, GetSuccessorsCompleteResponse, GetSuccessorsPartialResponse,
            InitPayload, Network,
        },
    };
    use bitcoin::BlockHeader;

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

        init(InitPayload {
            stability_threshold: 0,
            network,
            blocks_source: None,
        });

        let block = BlockBuilder::with_prev_header(genesis_block(network).header()).build();

        let mut block_bytes = vec![];
        block.consensus_encode(&mut block_bytes).unwrap();

        runtime::set_successors_response(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks: vec![block_bytes],
                next: vec![],
            },
        ));

        // Fetch blocks.
        heartbeat().await;

        // Process response.
        heartbeat().await;

        // Assert that the block has been ingested.
        assert_eq!(with_state(state::main_chain_height), 1);

        // The UTXO set hasn't been updated with the genesis block yet.
        assert_eq!(with_state(|s| s.utxos.next_height), 0);

        // Ingest the stable block (the genesis block) to the UTXO set.
        heartbeat().await;

        // Assert that the block has been ingested.
        assert_eq!(with_state(state::main_chain_height), 1);

        // The UTXO set has been updated with the genesis block.
        assert_eq!(with_state(|s| s.utxos.next_height), 1);
    }

    #[async_std::test]
    async fn time_slices_large_blocks() {
        let network = Network::Regtest;

        init(InitPayload {
            stability_threshold: 0,
            network,
            blocks_source: None,
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

        runtime::set_successors_response(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks,
                next: vec![],
            },
        ));

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
        let partial_block = with_state(|s| s.utxos.partial_stable_block.clone().unwrap());
        assert_eq!(partial_block.block, block_1);
        assert_eq!(partial_block.next_tx_idx, 2);
        assert_eq!(partial_block.next_input_idx, 1);
        assert_eq!(partial_block.next_output_idx, 0);

        // Ingest more stable blocks.
        runtime::performance_counter_reset();
        heartbeat().await;

        // Assert that execution has been paused. Ingested 3 more txs in block_1.
        let partial_block = with_state(|s| s.utxos.partial_stable_block.clone().unwrap());
        assert_eq!(partial_block.block, block_1);
        assert_eq!(partial_block.next_tx_idx, 5);
        assert_eq!(partial_block.next_input_idx, 1);
        assert_eq!(partial_block.next_output_idx, 0);

        // Only the genesis block has been fully processed, so the stable height is one.
        assert_eq!(with_state(|s| s.utxos.next_height), 1);

        // Ingest more stable blocks.
        runtime::performance_counter_reset();
        heartbeat().await;

        // Time slicing is complete.
        assert!(with_state(|s| s.utxos.partial_stable_block.is_none()));

        // Assert that the blocks have been ingested.
        assert_eq!(with_state(state::main_chain_height), 2);

        // The stable height is now updated to include `block_1`.
        assert_eq!(with_state(|s| s.utxos.next_height), 2);
    }

    #[async_std::test]
    async fn time_slices_large_transactions() {
        let network = Network::Regtest;
        // The number of inputs/outputs in a transaction.
        let tx_cardinality = 6;

        init(InitPayload {
            stability_threshold: 0,
            network,
            blocks_source: None,
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
            tx_2 = tx_2.with_input(crate::types::OutPoint {
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

        runtime::set_successors_response(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks,
                next: vec![],
            },
        ));

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
            PartialStableBlock::new(block_1.clone(), 0, 1, 2),
            PartialStableBlock::new(block_1.clone(), 0, 1, 5),
            PartialStableBlock::new(block_2.clone(), 0, 2, 0),
            PartialStableBlock::new(block_2.clone(), 0, 5, 0),
            PartialStableBlock::new(block_2.clone(), 0, 6, 2),
            PartialStableBlock::new(block_2.clone(), 0, 6, 5),
        ];

        for expected_state in expected_states.into_iter() {
            // Ingest stable blocks.
            runtime::performance_counter_reset();
            heartbeat().await;

            // Assert that execution has been paused.
            let partial_block = with_state(|s| s.utxos.partial_stable_block.clone().unwrap());
            assert_eq!(partial_block.block, expected_state.block);
            assert_eq!(partial_block.next_tx_idx, expected_state.next_tx_idx);
            assert_eq!(partial_block.next_input_idx, expected_state.next_input_idx);
            assert_eq!(
                partial_block.next_output_idx,
                expected_state.next_output_idx
            );
        }

        // Assert ingestion has finished.
        runtime::performance_counter_reset();
        heartbeat().await;
        with_state(|s| assert_eq!(s.utxos.partial_stable_block, None));

        // Assert that the blocks have been ingested.
        assert_eq!(with_state(state::main_chain_height), 3);

        // The stable height is now updated to include `block_1` and `block_2`.
        assert_eq!(with_state(|s| s.utxos.next_height), 3);

        // Query the balance, expecting address 1 to be empty and address 2 to be non-empty.
        assert_eq!(
            crate::get_balance(crate::types::GetBalanceRequest {
                address: address_1.to_string(),
                min_confirmations: None
            }),
            0
        );

        assert_eq!(
            crate::get_balance(crate::types::GetBalanceRequest {
                address: address_2.to_string(),
                min_confirmations: None
            }),
            tx_cardinality as u64 * 1000
        );
    }

    #[async_std::test]
    async fn fetches_and_processes_responses_paginated() {
        let network = Network::Regtest;

        init(InitPayload {
            stability_threshold: 0,
            network,
            blocks_source: None,
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
        runtime::set_successors_response(GetSuccessorsResponse::Partial(
            GetSuccessorsPartialResponse {
                partial_block: block_bytes[0..40].to_vec(),
                next: vec![],
                remaining_follow_ups: 2,
            },
        ));

        // Fetch blocks (initial response).
        heartbeat().await;

        // Fetch blocks (second page).
        runtime::set_successors_response(GetSuccessorsResponse::FollowUp(
            block_bytes[40..80].to_vec(),
        ));
        heartbeat().await;

        // Fetch blocks (third page).
        runtime::set_successors_response(GetSuccessorsResponse::FollowUp(
            block_bytes[80..].to_vec(),
        ));
        heartbeat().await;

        // The response hasn't been fully processed yet, so the balance should still be zero.
        assert_eq!(
            crate::get_balance(crate::types::GetBalanceRequest {
                address: address.to_string(),
                min_confirmations: None
            }),
            0
        );

        // Process response.
        heartbeat().await;

        // Query the balance, validating the block was processed.
        assert_eq!(
            crate::get_balance(crate::types::GetBalanceRequest {
                address: address.to_string(),
                min_confirmations: None
            }),
            1000
        );
    }
}
