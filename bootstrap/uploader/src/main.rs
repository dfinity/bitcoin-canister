//! A canister that writes data to stable memory.
use ic_cdk::api::stable;
use ic_cdk_macros::{init, query, update};
use std::{cell::RefCell, cmp::min, collections::BTreeSet};
use uploader::*;

// Load the chunks from the $CHUNK_HASHES_PATH environment variable that's set at build time.
include!(concat!(env!("OUT_DIR"), "/chunk_hashes.rs"));

thread_local! {
    // A set containing the indices of chunks that have not yet been uploaded.
    // An index here refers to the index of the Wasm page in stable memory where the chunk begins.
    static MISSING_CHUNKS: RefCell<BTreeSet<u64>> = RefCell::new(BTreeSet::new());
}

#[init]
fn init(state_size: u64) {
    // Grow the stable memory to the given size.
    stable::stable_grow(state_size).expect("cannot grow stabe memory");

    // Initialize the set of missing chunks.
    MISSING_CHUNKS.with(|mr| {
        mr.replace(
            (0..state_size)
                .step_by(CHUNK_SIZE_IN_PAGES as usize)
                .collect(),
        )
    });
}

#[update]
fn upload_chunk(chunk_start: u64, bytes: Vec<u8>) {
    // Verify the chunk is one of the missing chunks.
    if !MISSING_CHUNKS.with(|mr| mr.borrow().contains(&chunk_start)) {
        panic!(
            "invalid chunk or chunk is already uploaded: {}",
            chunk_start
        );
    }

    // Verify the length of the chunk is correct.
    let expected_end_chunk = min(chunk_start + CHUNK_SIZE_IN_PAGES, stable::stable_size());
    let expected_bytes_length = ((expected_end_chunk - chunk_start) * PAGE_SIZE_IN_BYTES) as usize;
    if expected_bytes_length != bytes.len() {
        panic!(
            "expected chunk to be {} bytes but found {} bytes",
            expected_bytes_length,
            bytes.len()
        );
    }

    // Verify that the hash of `bytes` matches some hash that we expect.
    let expected_hash = CHUNK_HASHES[(chunk_start / CHUNK_SIZE_IN_PAGES) as usize];
    let actual_hash = {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(&*bytes);
        let digest_bytes = hasher.finalize();
        hex::encode(&digest_bytes)
    };
    if actual_hash != expected_hash {
        panic!(
            "Expected digest {} but found {}. bytes snippet {:?}",
            expected_hash,
            actual_hash,
            &bytes[0..100]
        );
    }

    // Write the chunk.
    let offset = chunk_start * PAGE_SIZE_IN_BYTES;
    stable::stable_write(offset, &bytes);

    MISSING_CHUNKS.with(|mr| mr.borrow_mut().remove(&chunk_start));
}

// Returns the missing chunks indices.
#[query]
fn get_missing_chunk_indices() -> Vec<u64> {
    MISSING_CHUNKS.with(|mr| mr.borrow().iter().cloned().collect())
}

fn main() {}
