use crate::{
    get_balance, get_utxos, heartbeat, runtime,
    state::PartialStableBlock,
    store::main_chain_height,
    types::{
        BlockBlob, GetBalanceRequest, GetSuccessorsCompleteResponse, GetSuccessorsResponse,
        GetUtxosRequest, Network,
    },
    utxoset::DUPLICATE_TX_IDS,
    with_state,
};
use crate::{init, test_utils::random_p2pkh_address, InitPayload};
use bitcoin::blockdata::constants::genesis_block;
use bitcoin::Block;
use bitcoin::{
    consensus::{Decodable, Encodable},
    BlockHash, Txid,
};
use byteorder::{LittleEndian, ReadBytesExt};
use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};
use ic_btc_types::{GetUtxosResponse, UtxosFilter};
use ic_btc_types::{OutPoint, Utxo};
use std::fs::File;
use std::str::FromStr;
use std::{collections::HashMap, io::BufReader, path::PathBuf};

async fn process_chain(network: Network, blocks_file: &str, num_blocks: u32) {
    let mut chain: Vec<Block> = vec![];

    let mut blocks: HashMap<BlockHash, Block> = HashMap::new();

    let mut blk_file = BufReader::new(
        File::open(PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(blocks_file))
            .unwrap(),
    );

    loop {
        let magic = match blk_file.read_u32::<LittleEndian>() {
            Err(_) => break,
            Ok(magic) => {
                if magic == 0 {
                    // Reached EOF
                    break;
                }
                magic
            }
        };

        assert_eq!(
            magic,
            match network {
                Network::Mainnet => 0xD9B4BEF9,
                Network::Testnet | Network::Regtest => 0x0709110B,
            }
        );

        let _block_size = blk_file.read_u32::<LittleEndian>().unwrap();

        let block = Block::consensus_decode(&mut blk_file).unwrap();

        blocks.insert(block.header.prev_blockhash, block);
    }

    println!("# blocks in file: {}", blocks.len());

    // Build the chain
    chain.push(
        blocks
            .remove(&genesis_block(network.into()).block_hash())
            .unwrap(),
    );
    for _ in 1..num_blocks {
        let next_block = blocks.remove(&chain[chain.len() - 1].block_hash()).unwrap();
        chain.push(next_block);
    }

    println!("Built chain with length: {}", chain.len());

    // Map the blocks into responses that are given to the hearbeat.
    let responses: Vec<_> = chain
        .into_iter()
        .map(|block| {
            let mut block_bytes = vec![];
            Block::consensus_encode(&block, &mut block_bytes).unwrap();
            GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
                blocks: vec![block_bytes],
                next: vec![],
            })
        })
        .collect();

    runtime::set_successors_responses(responses);

    // Run the heartbeat until we process all the blocks.
    let mut i = 0;
    loop {
        runtime::performance_counter_reset();
        heartbeat().await;

        if i % 1000 == 0 {
            // The `main_chain_height` call is a bit expensive, so we only check every once
            // in a while.
            if with_state(main_chain_height) == num_blocks {
                break;
            }
        }

        i += 1;
    }
}

#[async_std::test]
async fn mainnet_100k_blocks() {
    crate::init(crate::InitPayload {
        stability_threshold: 10,
        network: Network::Mainnet,
        blocks_source: None,
    });

    // Set a reasonable performance counter step to trigger time-slicing.
    runtime::set_performance_counter_step(100_000);

    process_chain(
        Network::Mainnet,
        "test-data/mainnet_100k_blocks.dat",
        100_000,
    )
    .await;

    // Validate we've ingested all the blocks.
    assert_eq!(with_state(main_chain_height), 100_000);

    let mut total_supply = 0;
    crate::with_state(|state| {
        for (_, (v, _)) in state.utxos.utxos.iter() {
            total_supply += v.value;
        }

        // NOTE: The duplicate transactions cause us to lose some of the supply,
        // which we deduct in this assertion.
        assert_eq!(
            ((state.utxos.next_height as u64) - DUPLICATE_TX_IDS.len() as u64) * 5000000000,
            total_supply
        );
    });

    // Check some random addresses that the balance is correct:

    // https://blockexplorer.one/bitcoin/mainnet/address/1PgZsaGjvssNCqHHisshLoCFeUjxPhutTh
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1PgZsaGjvssNCqHHisshLoCFeUjxPhutTh".to_string(),
            min_confirmations: None
        }),
        4000000
    );

    assert_eq!(
        get_utxos(GetUtxosRequest {
            address: "1PgZsaGjvssNCqHHisshLoCFeUjxPhutTh".to_string(),
            filter: None
        }),
        GetUtxosResponse {
            utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: Txid::from_str(
                        "1a592a31c79f817ed787b6acbeef29b0f0324179820949d7da6215f0f4870c42",
                    )
                    .unwrap()
                    .to_vec(),
                    vout: 1,
                },
                value: 4000000,
                height: 75361,
            }],
            // The tip should be the block hash at height 100,000
            // https://bitcoinchain.com/block_explorer/block/100000/
            tip_block_hash: BlockHash::from_str(
                "000000000003ba27aa200b1cecaad478d2b00432346c3f1f3986da1afd33e506"
            )
            .unwrap()
            .to_vec(),
            tip_height: 100_000,
            next_page: None,
        }
    );

    // https://blockexplorer.one/bitcoin/mainnet/address/12tGGuawKdkw5NeDEzS3UANhCRa1XggBbK
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "12tGGuawKdkw5NeDEzS3UANhCRa1XggBbK".to_string(),
            min_confirmations: None
        }),
        500000000
    );

    assert_eq!(
        get_utxos(GetUtxosRequest {
            address: "12tGGuawKdkw5NeDEzS3UANhCRa1XggBbK".to_string(),
            filter: None
        }),
        GetUtxosResponse {
            utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: Txid::from_str(
                        "3371b3978e7285d962fd54656aca6b3191135a1db838b5c689b8a44a7ede6a31",
                    )
                    .unwrap()
                    .to_vec(),
                    vout: 0,
                },
                value: 500000000,
                height: 66184,
            }],
            // The tip should be the block hash at height 100,000
            // https://bitcoinchain.com/block_explorer/block/100000/
            tip_block_hash: BlockHash::from_str(
                "000000000003ba27aa200b1cecaad478d2b00432346c3f1f3986da1afd33e506"
            )
            .unwrap()
            .to_vec(),
            tip_height: 100_000,
            next_page: None,
        }
    );

    // This address spent its BTC at height 99,996. At 0 confirmations
    // (height 100,000) it should have no BTC.
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro".to_string(),
            min_confirmations: None
        }),
        0
    );

    // At 10 confirmations it should have its BTC.
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro".to_string(),
            min_confirmations: Some(10)
        }),
        48_0000_0000
    );

    // At 6 confirmations it should have its BTC.
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro".to_string(),
            min_confirmations: Some(6)
        }),
        48_0000_0000
    );

    assert_eq!(
        get_utxos(GetUtxosRequest {
            address: "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro".to_string(),
            filter: Some(UtxosFilter::MinConfirmations(6))
        }),
        GetUtxosResponse {
            utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: Txid::from_str(
                        "2bdd8506980479fb57d848ddbbb29831b4d468f9dc5d572ccdea69edec677ed6",
                    )
                    .unwrap()
                    .to_vec(),
                    vout: 1,
                },
                value: 48_0000_0000,
                height: 96778,
            }],
            // The tip should be the block hash at height 99,995
            // https://blockchair.com/bitcoin/block/99995
            tip_block_hash: BlockHash::from_str(
                "00000000000471d4db69f006cefc583aee6dec243d63c6a09cd5c02e0ef52523",
            )
            .unwrap()
            .to_vec(),
            tip_height: 99_995,
            next_page: None,
        }
    );

    // At 5 confirmations the BTC is spent.
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro".to_string(),
            min_confirmations: Some(5)
        }),
        0
    );

    // The BTC is spent to the following two addresses.
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1NhzJ8bsdmGK39vSJtdQw3R2HyNtUmGxcr".to_string(),
            min_confirmations: Some(5),
        }),
        3_4500_0000
    );

    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "13U77vKQcTjpZ7gww4K8Nreq2ffGBQKxmr".to_string(),
            min_confirmations: Some(5)
        }),
        44_5500_0000
    );

    // And these addresses should have a balance of zero before that height.
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1NhzJ8bsdmGK39vSJtdQw3R2HyNtUmGxcr".to_string(),
            min_confirmations: Some(6),
        }),
        0
    );

    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "13U77vKQcTjpZ7gww4K8Nreq2ffGBQKxmr".to_string(),
            min_confirmations: Some(6),
        }),
        0
    );
}

#[async_std::test]
async fn testnet_10k_blocks() {
    crate::init(crate::InitPayload {
        stability_threshold: 2,
        network: Network::Testnet,
        blocks_source: None,
    });

    // Set a reasonable performance counter step to trigger time-slicing.
    runtime::set_performance_counter_step(100_000);

    process_chain(Network::Testnet, "test-data/testnet_10k_blocks.dat", 10_000).await;

    // Validate we've ingested all the blocks.
    assert_eq!(with_state(main_chain_height), 10_000);

    // Verify the total supply
    let mut total_supply = 0;
    crate::with_state(|state| {
        for (_, (v, _)) in state.utxos.utxos.iter() {
            total_supply += v.value;
        }

        assert_eq!(state.utxos.next_height as u64 * 5000000000, total_supply);
    });
}

#[async_std::test]
async fn time_slices_large_block_with_multiple_transactions() {
    let network = Network::Regtest;
    init(InitPayload {
        stability_threshold: 0,
        network,
        blocks_source: None,
    });

    let address_1 = random_p2pkh_address(network);
    let address_2 = random_p2pkh_address(network);

    let tx_1 = TransactionBuilder::coinbase()
        .with_output(&address_1, 1000)
        .with_output(&address_1, 1000)
        .build();

    let tx_2 = TransactionBuilder::new()
        .with_output(&address_2, 1000)
        .with_output(&address_2, 1000)
        .build();

    let block_1 = BlockBuilder::with_prev_header(genesis_block(network.into()).header)
        .with_transaction(tx_1)
        .with_transaction(tx_2)
        .build();

    // An additional block so that the previous block is ingested into the stable UTXO set.
    let block_2 = BlockBuilder::with_prev_header(block_1.header).build();

    // Serialize the blocks.
    let blocks: Vec<BlockBlob> = [block_1.clone(), block_2.clone()]
        .iter()
        .map(|block| {
            let mut block_bytes = vec![];
            block.consensus_encode(&mut block_bytes).unwrap();
            block_bytes
        })
        .collect();

    runtime::set_successors_response(GetSuccessorsResponse::Complete(
        GetSuccessorsCompleteResponse {
            blocks,
            next: vec![],
        },
    ));

    // Set a large step for the performance_counter to exceed the instructions limit quickly.
    // This value allows ingesting 2 transactions inputs/outputs per round.
    runtime::set_performance_counter_step(1_500_000_000);

    // Fetch blocks.
    heartbeat().await;

    // Process response.
    heartbeat().await;

    // Assert that the block has been ingested.
    assert_eq!(with_state(main_chain_height), 2);

    // Run the heartbeat a few rounds to ingest the blocks.
    let expected_states = vec![
        PartialStableBlock::new(block_1.clone(), 0, 1, 1),
        PartialStableBlock::new(block_1.clone(), 1, 1, 1),
    ];

    for expected_state in expected_states.into_iter() {
        // Ingest stable blocks.
        runtime::performance_counter_reset();
        heartbeat().await;

        // Assert that execution has been paused.
        let partial_block = with_state(|s| s.utxos.partial_stable_block.clone().unwrap());
        assert_eq!(partial_block.block, expected_state.block);
        assert_eq!(partial_block.next_tx_idx, expected_state.next_tx_idx);
        assert_eq!(partial_block.next_input_idx, expected_state.next_input_idx);
        assert_eq!(
            partial_block.next_output_idx,
            expected_state.next_output_idx
        );
    }

    // Assert ingestion has finished.
    runtime::performance_counter_reset();
    heartbeat().await;

    // The stable height is now updated to include `block_1`.
    assert_eq!(with_state(|s| s.utxos.next_height), 2);

    // Query the balance, expecting address 1 to be empty and address 2 to be non-empty.
    assert_eq!(
        crate::get_balance(crate::types::GetBalanceRequest {
            address: address_1.to_string(),
            min_confirmations: None
        }),
        2000
    );

    assert_eq!(
        crate::get_balance(crate::types::GetBalanceRequest {
            address: address_2.to_string(),
            min_confirmations: None
        }),
        2000
    );
}
