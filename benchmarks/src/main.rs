use bitcoin::consensus::Decodable;
use bitcoin::constants::genesis_block;
use bitcoin::{block::Header, consensus::Encodable, Block as BitcoinBlock};
use canbench_rs::{bench, bench_fn, BenchResult};
use ic_btc_canister::{types::BlockHeaderBlob, with_state_mut, CanisterArg};
use ic_btc_interface::{InitConfig, Network};
use ic_btc_test_utils::{build_regtest_chain, BlockBuilder, TransactionBuilder};
use ic_btc_types::Block;
use ic_cdk::init;
use std::cell::RefCell;
use std::str::FromStr;

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

    // Set mock time to avoid timestamp validation failure due to blocks appearing to be > 2 hours
    // in the future.
    let june_2025 = (55.5 * 365.25 * 24.0 * 60.0 * 60.0) as u64;
    ic_btc_canister::runtime::mock_time::set_mock_time_secs(june_2025);
}

// Benchmarks inserting the first 300 blocks of the Bitcoin testnet.
#[bench(raw)]
fn insert_300_blocks() -> BenchResult {
    ic_btc_canister::init(CanisterArg::Init(InitConfig {
        network: Some(Network::Testnet),
        stability_threshold: Some(144),
        ..Default::default()
    }));

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
    ic_btc_canister::init(CanisterArg::Init(InitConfig {
        network: Some(Network::Testnet),
        stability_threshold: Some(3000),
        ..Default::default()
    }));

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

    ic_btc_canister::init(CanisterArg::Init(InitConfig {
        network: Some(Network::Testnet),
        ..Default::default()
    }));

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
    ic_btc_canister::init(CanisterArg::Init(InitConfig {
        network: Some(Network::Testnet),
        ..Default::default()
    }));

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
fn insert_block_with_10k_transactions() -> BenchResult {
    bench_insert_block(10_000)
}

#[bench(raw)]
fn insert_block_with_1k_transactions() -> BenchResult {
    bench_insert_block(1_000)
}

#[bench(raw)]
fn pre_upgrade_with_many_unstable_blocks() -> BenchResult {
    let blocks = build_regtest_chain(3000, 100);

    ic_btc_canister::init(CanisterArg::Init(InitConfig {
        network: Some(Network::Regtest),
        ..Default::default()
    }));

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

fn bench_insert_block(num_transactions: u32) -> BenchResult {
    /// Create a chain of 2 blocks after genesis.
    ///
    /// 1st block:
    /// * 1 coinbase transaction with `tx_cardinality` outputs
    ///
    /// 2nd block:
    /// * `tx_cardinality` transactions consuming the previous outputs
    fn mini_chain(tx_cardinality: u32) -> [Block; 2] {
        const ADDRESS_1: &str = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8";
        const ADDRESS_2: &str = "bcrt1qxp8ercrmfxlu0s543najcj6fe6267j97tv7rgf";

        let address_1 = bitcoin::Address::from_str(ADDRESS_1)
            .unwrap()
            .assume_checked();
        let address_2 = bitcoin::Address::from_str(ADDRESS_2)
            .unwrap()
            .assume_checked();

        // Transaction 1: A coinbase tx with `tx_cardinality` inputs, each giving 1 Satoshi to
        // address 1.
        let mut tx_1 = TransactionBuilder::coinbase();
        for i in 0..tx_cardinality {
            tx_1 = tx_1.with_output(&address_1, 1).with_lock_time(i)
        }
        let tx_1 = tx_1.build();
        let tx_1_id: bitcoin::Txid = tx_1.compute_txid();

        // Transaction 2: Consume all the outputs of transaction 1 *in reverse order* and create
        // similar outputs for address 2.
        let mut tx_2 = TransactionBuilder::new();
        for i in (0..tx_cardinality).rev() {
            tx_2 = tx_2.with_input(
                bitcoin::OutPoint {
                    vout: i,
                    txid: tx_1_id,
                },
                None,
            );
        }
        for i in 0..tx_cardinality {
            tx_2 = tx_2.with_output(&address_2, 1).with_lock_time(i);
        }
        let tx_2 = tx_2.build();

        let genesis = genesis_block(bitcoin::Network::Regtest);
        let block_1 = BlockBuilder::with_prev_header(genesis.header)
            .with_transaction(tx_1)
            .build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header)
            .with_transaction(TransactionBuilder::coinbase().build())
            .with_transaction(tx_2)
            .build();
        [Block::new(block_1), Block::new(block_2)]
    }
    let [block_1, block_2] = mini_chain(num_transactions);

    ic_btc_canister::init(CanisterArg::Init(InitConfig {
        network: Some(Network::Regtest),
        ..Default::default()
    }));

    with_state_mut(|s| {
        ic_btc_canister::state::insert_block(s, block_1).unwrap();
    });

    bench_fn(|| {
        with_state_mut(|s| {
            ic_btc_canister::state::insert_block(s, block_2).unwrap();
        });
    })
}

fn main() {}
