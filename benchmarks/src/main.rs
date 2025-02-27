use bitcoin::consensus::Decodable;
use bitcoin::{block::Header, consensus::Encodable, Block as BitcoinBlock};
use canbench_rs::{bench, bench_fn, BenchResult};
use ic_btc_canister::{types::BlockHeaderBlob, with_state_mut};
use ic_btc_interface::{InitConfig, Network};
use ic_btc_test_utils::build_regtest_chain;
use ic_btc_types::Block;
use ic_cdk_macros::init;
use std::cell::RefCell;

thread_local! {
    static TESTNET_BLOCKS: RefCell<Vec<Block>> =  const { RefCell::new(vec![])};
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
                    Block::new(BitcoinBlock::consensus_decode(&mut block_bytes.as_slice()).unwrap())
                })
                .collect(),
        );
    });
}

// Benchmarks inserting the first 300 blocks of the Bitcoin testnet.
#[bench(raw)]
fn insert_300_blocks() -> BenchResult {
    ic_btc_canister::init(InitConfig {
        network: Some(Network::Testnet),
        stability_threshold: Some(144),
        ..Default::default()
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
    ic_btc_canister::init(InitConfig {
        network: Some(Network::Testnet),
        stability_threshold: Some(3000),
        ..Default::default()
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

    ic_btc_canister::init(InitConfig {
        network: Some(Network::Testnet),
        ..Default::default()
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
            Header::consensus_encode(blocks[i as usize].header(), &mut block_header_blob).unwrap();
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
    ic_btc_canister::init(InitConfig {
        network: Some(Network::Testnet),
        ..Default::default()
    });

    // Compute the next block headers.
    let next_block_headers = TESTNET_BLOCKS.with(|b| {
        let blocks = b.borrow();
        let mut next_block_headers = vec![];
        for i in 0..1000 {
            let mut block_header_blob = vec![];
            Header::consensus_encode(blocks[i as usize].header(), &mut block_header_blob).unwrap();
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

#[bench(raw)]
fn pre_upgrade_with_many_unstable_blocks() -> BenchResult {
    let blocks = build_regtest_chain(3000, 100);

    ic_btc_canister::init(InitConfig {
        network: Some(Network::Regtest),
        ..Default::default()
    });

    // Insert the blocks.
    with_state_mut(|s| {
        for block in blocks.into_iter().skip(1) {
            ic_btc_canister::state::insert_block(s, block).unwrap();
        }
    });

    bench_fn(|| {
        ic_btc_canister::pre_upgrade();
    })
}

fn main() {}
