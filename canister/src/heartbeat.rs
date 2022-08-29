use crate::{
    runtime::{call_get_successors, print},
    store,
    types::{GetSuccessorsRequest, GetSuccessorsResponse},
};
use crate::{with_state, with_state_mut};
use bitcoin::consensus::Decodable;
use bitcoin::Block;

/// The heartbeat of the Bitcoin canister.
///
/// The heartbeat fetches new blocks from the bitcoin network and inserts them into the state.
pub async fn heartbeat() {
    if ingest_stable_blocks_into_utxoset() {
        // Exit the heartbeat if stable blocks had been processed as a precaution to not
        // exceed the instructions limit.
        return;
    }

    // Only fetch new blocks if there isn't a request in progress and there is no
    // response to process.
    let should_fetch_blocks = with_state(|s| {
        !s.syncing_state.is_fetching_blocks && s.syncing_state.response_to_process.is_none()
    });

    if should_fetch_blocks {
        return fetch_blocks().await;
    }

    maybe_process_response();
}

async fn fetch_blocks() {
    // A lock to ensure the heartbeat only sends one request at a time.
    with_state_mut(|s| {
        s.syncing_state.is_fetching_blocks = true;
    });

    // Request additional blocks.
    let request = get_successors_request();
    print(&format!("Sending request: {:?}", request));

    let response: Result<(GetSuccessorsResponse,), _> =
        call_get_successors(with_state(|s| s.blocks_source), request).await;

    print(&format!("Received response: {:?}", response));

    // Release the heartbeat lock and save the response.
    with_state_mut(|s| {
        s.syncing_state.is_fetching_blocks = false;

        match response {
            Ok((response,)) => {
                s.syncing_state.response_to_process = Some(response);
            }
            Err((code, msg)) => {
                print(&format!("Error fetching blocks: [{:?}] {}", code, msg));
                s.syncing_state.response_to_process = None;
            }
        }
    });
}

fn ingest_stable_blocks_into_utxoset() -> bool {
    with_state_mut(store::ingest_stable_blocks_into_utxoset)
}

// Process a `GetSuccessorsResponse` if one is available.
fn maybe_process_response() {
    with_state_mut(|state| {
        let response_to_process = state.syncing_state.response_to_process.take();
        if let Some(response) = response_to_process {
            let blocks = response.blocks;
            for block in blocks.into_iter() {
                // TODO(EXC-1215): Gracefully handle the errors here.
                let block = Block::consensus_decode(block.as_slice()).unwrap();
                store::insert_block(state, block).unwrap();
            }
        }
    });
}

// Retrieves a `GetSuccessorsRequest` to send to the adapter.
fn get_successors_request() -> GetSuccessorsRequest {
    with_state(|state| {
        let mut processed_block_hashes: Vec<Vec<u8>> = store::get_unstable_blocks(state)
            .iter()
            .map(|b| b.block_hash().to_vec())
            .collect();

        // This is safe as there will always be at least 1 unstable block.
        let anchor = processed_block_hashes.remove(0);

        GetSuccessorsRequest {
            anchor,
            processed_block_hashes,
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        runtime,
        state::PartialStableBlock,
        test_utils::random_p2pkh_address,
        types::{BlockBlob, Network},
    };
    use bitcoin::{
        blockdata::constants::genesis_block, consensus::Encodable, Address, Block, BlockHeader,
    };
    use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};

    fn build_block(prev_header: BlockHeader, address: Address, num_transactions: u128) -> Block {
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

        crate::init(crate::InitPayload {
            stability_threshold: 0,
            network,
            blocks_source: None,
        });

        let block = BlockBuilder::with_prev_header(genesis_block(network.into()).header).build();

        let mut block_bytes = vec![];
        block.consensus_encode(&mut block_bytes).unwrap();

        crate::runtime::set_successors_response(GetSuccessorsResponse {
            blocks: vec![block_bytes],
            next: vec![],
        });

        // Fetch blocks.
        heartbeat().await;

        // Process response.
        heartbeat().await;

        // Assert that the block has been ingested.
        assert_eq!(with_state(store::main_chain_height), 1);

        // The UTXO set hasn't been updated with the genesis block yet.
        assert_eq!(with_state(|s| s.height), 0);

        // Ingest the stable block (the genesis block) to the UTXO set.
        heartbeat().await;

        // Assert that the block has been ingested.
        assert_eq!(with_state(store::main_chain_height), 1);

        // The UTXO set has been updated with the genesis block.
        assert_eq!(with_state(|s| s.height), 1);
    }

    #[async_std::test]
    async fn time_slices_large_blocks() {
        let network = Network::Regtest;

        crate::init(crate::InitPayload {
            stability_threshold: 0,
            network,
            blocks_source: None,
        });

        // Setup a chain of two blocks.
        let address = random_p2pkh_address(network);
        let block_1 = build_block(genesis_block(network.into()).header, address.clone(), 10);
        let block_2 = build_block(block_1.header, address, 1);

        // Serialize the blocks.
        let blocks: Vec<BlockBlob> = [block_1.clone(), block_2]
            .iter()
            .map(|block| {
                let mut block_bytes = vec![];
                block.consensus_encode(&mut block_bytes).unwrap();
                block_bytes
            })
            .collect();

        crate::runtime::set_successors_response(GetSuccessorsResponse {
            blocks,
            next: vec![],
        });

        // Set a large step for the performance_counter to exceed the instructions limit quickly.
        runtime::set_performance_counter_step(1_000_000_000);

        // Fetch blocks.
        heartbeat().await;

        // Process response.
        heartbeat().await;

        // Assert that the blocks have been ingested.
        assert_eq!(with_state(store::main_chain_height), 2);

        // Ingest stable blocks.
        runtime::performance_counter_reset();
        heartbeat().await;

        // Assert that execution has been paused.
        // Wrote the genesis block + 3 transactions of block_1 into the UTXO set.
        assert_eq!(
            with_state(|s| s.syncing_state.partial_stable_block.clone().unwrap()),
            PartialStableBlock {
                block: block_1.clone(),
                txs_processed: 3
            }
        );

        // Ingest more stable blocks.
        runtime::performance_counter_reset();
        heartbeat().await;

        // Assert that execution has been paused. Added more transactions in block_1
        // into the UTXO set.
        assert_eq!(
            with_state(|s| s.syncing_state.partial_stable_block.clone().unwrap()),
            PartialStableBlock {
                block: block_1,
                txs_processed: 7
            }
        );

        // Only the genesis block has been fully processed, so the stable height is one.
        assert_eq!(with_state(|s| s.height), 1);

        // Ingest more stable blocks.
        runtime::performance_counter_reset();
        heartbeat().await;

        // Time slicing is complete.
        assert!(with_state(|s| s
            .syncing_state
            .partial_stable_block
            .is_none()));

        // Assert that the blocks have been ingested.
        assert_eq!(with_state(store::main_chain_height), 2);

        // The stable height is now updated to include `block_1`.
        assert_eq!(with_state(|s| s.height), 2);
    }
}
