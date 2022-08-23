use crate::{
    store,
    types::{GetSuccessorsRequest, GetSuccessorsResponse},
};
use crate::{with_state, with_state_mut};
use bitcoin::consensus::Decodable;
use bitcoin::Block;
use ic_cdk::api::{call::call, print};

/// The heartbeat of the Bitcoin canister.
///
/// The heartbeat sends and processes `GetSuccessor` requests/responses, which
/// is needed to fetch new blocks from the network.
pub async fn heartbeat() {
    let is_locked = with_state(|s| s.heartbeat_in_progress);
    if is_locked {
        // Another heartbeat is already in progress.
        return;
    }

    // Lock the heartbeat method to prevent future heartbeats from running
    // until the lock is released.
    with_state_mut(|s| {
        s.heartbeat_in_progress = true;
    });

    // Request additional blocks.
    let request = get_successors_request();
    print(&format!("Sending request: {:?}", request));
    let response: Result<(GetSuccessorsResponse,), _> = call(
        with_state(|s| s.blocks_source),
        "bitcoin_get_successors",
        (request,),
    )
    .await;

    print(&format!("Received response: {:?}", response));

    // Release the heartbeat lock.
    with_state_mut(|s| {
        s.heartbeat_in_progress = false;
    });

    // TODO: Process the response in a separate heartbeat.
    // TODO: Gracefully handle errors instead of unwrapping.
    with_state_mut(|state| {
        let blocks = response.unwrap().0.blocks;
        for block in blocks.into_iter() {
            let block = Block::consensus_decode(block.as_slice()).unwrap();
            store::insert_block(state, block).unwrap();
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
