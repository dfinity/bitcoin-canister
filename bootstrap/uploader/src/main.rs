//! A canister that writes data to stable memory.
use ic_cdk::api::stable;
use ic_cdk_macros::{init, query, update};
use std::{cell::RefCell, cmp::min, collections::BTreeSet};
use uploader::*;

thread_local! {
    // A set containing the indices of chunks that have not yet been uploaded.
    // An index here refers to the index of the Wasm page in stable memory where the chunk begins.
    static MISSING_CHUNKS: RefCell<BTreeSet<u64>> = RefCell::new(BTreeSet::new());

    static CHUNK_HASHES: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

#[init]
fn init(initial_size: u64, chunk_hashes: Vec<String>) {
    // Grow the stable memory to the given size.
    stable::stable64_grow(initial_size).expect("cannot grow stabe memory");

    // Initialize the set of missing chunks.
    MISSING_CHUNKS.with(|mr| {
        mr.replace(
            (0..initial_size)
                .step_by(CHUNK_SIZE_IN_PAGES as usize)
                .collect(),
        )
    });

    CHUNK_HASHES.with(|ch| ch.replace(chunk_hashes));
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
    let expected_end_chunk = min(chunk_start + CHUNK_SIZE_IN_PAGES, stable::stable64_size());
    let expected_bytes_length = ((expected_end_chunk - chunk_start) * PAGE_SIZE_IN_BYTES) as usize;
    if expected_bytes_length != bytes.len() {
        panic!(
            "expected chunk to be {} bytes but found {} bytes",
            expected_bytes_length,
            bytes.len()
        );
    }

    // Verify that the hash of `bytes` matches some hash that we expect.
    let expected_hash =
        CHUNK_HASHES.with(|ch| ch.borrow()[(chunk_start / CHUNK_SIZE_IN_PAGES) as usize].clone());
    let actual_hash = sha256::digest(&*bytes);
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
    stable::stable64_write(offset, &bytes);

    MISSING_CHUNKS.with(|mr| mr.borrow_mut().remove(&chunk_start));
}

// Returns the missing chunks indices.
#[query]
fn get_missing_chunk_indices() -> Vec<u64> {
    MISSING_CHUNKS.with(|mr| mr.borrow().iter().cloned().collect())
}

fn main() {}
