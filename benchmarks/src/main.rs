use bitcoin::consensus::Decodable;
use bitcoin::{consensus::Encodable, Block as BitcoinBlock, BlockHeader};
use ic_btc_canister::{
    types::{Block, BlockHeaderBlob},
    with_state_mut,
};
use ic_btc_interface::{Config, Network};
use ic_cdk_macros::{init, query};
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
#[query]
fn insert_300_blocks() -> u64 {
    ic_btc_canister::init(Config {
        network: Network::Testnet,
        stability_threshold: 144,
        ..Config::default()
    });

    count_instructions(|| {
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
#[query]
fn get_metrics() -> u64 {
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

    count_instructions(|| {
        ic_btc_canister::get_metrics();
    })
}

// Benchmarks inserting 100 block headers into a tree containing 1000 blocks
#[query]
fn insert_block_headers() -> u64 {
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
            next_block_headers.push(BlockHeaderBlob::try_from(block_header_blob).unwrap());
        }

        next_block_headers
    });

    // Benchmark inserting the block headers.
    count_instructions(|| {
        with_state_mut(|s| {
            ic_btc_canister::state::insert_next_block_headers(s, next_block_headers.as_slice());
        });
    })
}

// Returns the number of instructions consumed by the given function.
fn count_instructions<R>(f: impl FnOnce() -> R) -> u64 {
    let start = ic_cdk::api::performance_counter(0);
    f();
    ic_cdk::api::performance_counter(0) - start
}

fn main() {}
