use bitcoin::consensus::Decodable;
use bitcoin::{consensus::Encodable, Block as BitcoinBlock, BlockHeader};
use canbench_rs::{bench, bench_fn, BenchResult};
use ic_btc_canister::{types::BlockHeaderBlob, with_state_mut};
use ic_btc_interface::{Config, Network};
use ic_btc_types::Block;
use ic_cdk_macros::init;
use std::cell::RefCell;

thread_local! {
    static TESTNET_BLOCKS: RefCell<Vec<Block>> =  RefCell::new(vec![]);
}

#[init]
fn init() {
    // Load the testnet blocks.
    TESTNET_BLOCKS.with(|blocks| {
        blocks.replace(
            include_str!("testnet_blocks.txt")
                .trim()
                .split('\n')
                .map(|block_hex| {
                    let block_bytes = hex::decode(block_hex).unwrap();
                    Block::new(BitcoinBlock::consensus_decode(block_bytes.as_slice()).unwrap())
                })
                .collect(),
        );
    });
}

// Benchmarks inserting the first 300 blocks of the Bitcoin testnet.
#[bench(raw)]
fn insert_300_blocks() -> BenchResult {
    ic_btc_canister::init(Config {
        network: Network::Testnet,
        stability_threshold: 144,
        ..Config::default()
    });

    bench_fn(|| {
        with_state_mut(|s| {
            for i in 0..300 {
                ic_btc_canister::state::insert_block(
                    s,
                    TESTNET_BLOCKS.with(|b| b.borrow()[i as usize].clone()),
                )
                .unwrap();
            }
        });
    })
}

// Benchmarks gettings the metrics when there are many unstable blocks..
#[bench(raw)]
fn get_metrics() -> BenchResult {
    ic_btc_canister::init(Config {
        network: Network::Testnet,
        stability_threshold: 3000,
        ..Config::default()
    });

    with_state_mut(|s| {
        for i in 0..3000 {
            ic_btc_canister::state::insert_block(
                s,
                TESTNET_BLOCKS.with(|b| b.borrow()[i as usize].clone()),
            )
            .unwrap();
        }
    });

    bench_fn(|| {
        ic_btc_canister::get_metrics();
    })
}

// Benchmarks inserting 100 block headers into a tree containing 1000 blocks
#[bench(raw)]
fn insert_block_headers() -> BenchResult {
    let blocks_to_insert = 1000;
    let block_headers_to_insert = 100;

    ic_btc_canister::init(Config {
        network: Network::Testnet,
        ..Config::default()
    });

    // Insert the blocks.
    with_state_mut(|s| {
        for i in 0..blocks_to_insert {
            ic_btc_canister::state::insert_block(
                s,
                TESTNET_BLOCKS.with(|b| b.borrow()[i as usize].clone()),
            )
            .unwrap();
        }
    });

    // Compute the next block headers.
    let next_block_headers = TESTNET_BLOCKS.with(|b| {
        let blocks = b.borrow();
        let mut next_block_headers = vec![];
        for i in blocks_to_insert..blocks_to_insert + block_headers_to_insert {
            let mut block_header_blob = vec![];
            BlockHeader::consensus_encode(blocks[i as usize].header(), &mut block_header_blob)
                .unwrap();
            next_block_headers.push(BlockHeaderBlob::from(block_header_blob));
        }

        next_block_headers
    });

    // Benchmark inserting the block headers.
    bench_fn(|| {
        with_state_mut(|s| {
            ic_btc_canister::state::insert_next_block_headers(s, next_block_headers.as_slice());
        });
    })
}

// Inserts the same block headers multiple times.
#[bench(raw)]
fn insert_block_headers_multiple_times() -> BenchResult {
    ic_btc_canister::init(Config {
        network: Network::Testnet,
        ..Config::default()
    });

    // Compute the next block headers.
    let next_block_headers = TESTNET_BLOCKS.with(|b| {
        let blocks = b.borrow();
        let mut next_block_headers = vec![];
        for i in 0..1000 {
            let mut block_header_blob = vec![];
            BlockHeader::consensus_encode(blocks[i as usize].header(), &mut block_header_blob)
                .unwrap();
            next_block_headers.push(BlockHeaderBlob::from(block_header_blob));
        }

        next_block_headers
    });

    // Benchmark inserting the block headers.
    bench_fn(|| {
        with_state_mut(|s| {
            for _ in 0..10 {
                ic_btc_canister::state::insert_next_block_headers(s, next_block_headers.as_slice());
            }
        });
    })
}

fn main() {}
