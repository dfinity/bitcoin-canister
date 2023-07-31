use bitcoin::consensus::Decodable;
use bitcoin::{
    blockdata::constants::genesis_block, Address, Block as BitcoinBlock, Network as BitcoinNetwork,
};
use ic_btc_canister::{types::Block, with_state_mut};
use ic_btc_interface::{Config, Network};
use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};
use ic_cdk::print;
use ic_cdk_macros::{init, query};
use std::cell::RefCell;
use std::str::FromStr;

thread_local! {
    static TESTNET_BLOCKS: RefCell<Vec<Block>> =  RefCell::new(vec![]);
}

//const ADDRESS_1: &str = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8";

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

#[query]
fn deep_unstable_chain_pre_upgrade() {
    ic_btc_canister::init(Config {
        network: Network::Testnet,
        stability_threshold: 1,
        ..Config::default()
    });

    with_state_mut(|s| {
        for i in 0..5_000 {
            ic_btc_canister::state::insert_block(
                s,
                TESTNET_BLOCKS.with(|b| b.borrow()[i as usize].clone()),
            )
            .unwrap();
        }
    });

    ic_btc_canister::pre_upgrade();
}

/*#[query]
async fn deep_unstable_chain_heartbeat() {
    print("Starting...");
    ic_btc_canister::init(Config {
        network: Network::Testnet,
        stability_threshold: 1,
        ..Config::default()
    });

    with_state_mut(|s| {
        let mut block = Block::new(
            BlockBuilder::with_prev_header(genesis_block(BitcoinNetwork::Testnet).header)
                .with_transaction(
                    TransactionBuilder::coinbase()
                        .with_output(&Address::from_str(ADDRESS_1).unwrap(), 1)
                        .build(),
                )
                .build(),
        );
        for i in 0..5_000 {
            print(&format!("i: {}", i));
            ic_btc_canister::state::insert_block(s, block.clone()).unwrap();
            block = Block::new(
                BlockBuilder::with_prev_header(*block.header())
                    .with_transaction(
                        TransactionBuilder::coinbase()
                            .with_output(&Address::from_str(ADDRESS_1).unwrap(), 1)
                            .build(),
                    )
                    .build(),
            );
        }
    });

    print("Running heartbeat...");
    ic_btc_canister::heartbeat().await;
    print("Heartbeat completed successfully.");
}*/

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

// Returns the number of instructions consumed by the given function.
fn count_instructions<R>(f: impl FnOnce() -> R) -> u64 {
    let start = ic_cdk::api::performance_counter(0);
    f();
    ic_cdk::api::performance_counter(0) - start
}

fn main() {
    /*use std::fs::File;
    use std::io::Write;
    let mut output = File::create("block_bytes.txt").unwrap();
    for block_hex in include_str!("testnet_blocks.txt").trim().split('\n') {
        let block_bytes = hex::decode(block_hex).unwrap();
        output.write_all(block_bytes.as_slice()).unwrap();
        writeln!(output, "").unwrap();
    }*/
}
