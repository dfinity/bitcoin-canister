use bitcoin::consensus::Decodable;
use bitcoin::Block as BitcoinBlock;
use ic_btc_canister::{
    types::Config,
    types::{Block, Network},
    with_state_mut,
};
use ic_cdk_macros::{init, query};
use std::cell::RefCell;

thread_local! {
    static TESTNET_BLOCKS: RefCell<Vec<Block>> =  RefCell::new(vec![]);
}

#[init]
fn init() {
    ic_btc_canister::init(Config {
        network: Network::Testnet,
        stability_threshold: 144,
        ..Config::default()
    });

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

// Returns the number of instructions consumed by the given function.
fn count_instructions<R>(f: impl FnOnce() -> R) -> u64 {
    let start = ic_cdk::api::performance_counter(0);
    f();
    ic_cdk::api::performance_counter(0) - start
}

fn main() {}
