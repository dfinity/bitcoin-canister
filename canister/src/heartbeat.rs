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
/// The heartbeat fetches new blocks from the bitcoin network and inserts them into the state.
pub async fn heartbeat() {
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
    let response: Result<(GetSuccessorsResponse,), _> = call(
        with_state(|s| s.blocks_source),
        "bitcoin_get_successors",
        (request,),
    )
    .await;

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
