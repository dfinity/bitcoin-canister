use crate::{
    api::{get_balance, get_current_fee_percentiles, get_utxos},
    genesis_block, get_blockchain_info, heartbeat, init,
    runtime::{self, GetSuccessorsReply},
    state::main_chain_height,
    test_utils::{BlockBuilder, BlockChainBuilder, TransactionBuilder},
    types::{
        into_bitcoin_network, BlockBlob, BlockHeaderBlob, GetBalanceRequest,
        GetSuccessorsCompleteResponse, GetSuccessorsResponse, GetUtxosRequest,
    },
    utxo_set::{IngestingBlock, DUPLICATE_TX_IDS},
    verify_synced, with_state, SYNCED_THRESHOLD,
};
use bitcoin::{
    block::Header,
    consensus::{Decodable, Encodable},
    p2p::Magic,
    Block as BitcoinBlock, Network as BitcoinNetwork,
};
use byteorder::{LittleEndian, ReadBytesExt};
use ic_btc_interface::{Flag, GetUtxosResponse, InitConfig, Network, Txid, UtxosFilter};
use ic_btc_interface::{OutPoint, Utxo};
use ic_btc_test_utils::random_p2pkh_address;
use ic_btc_types::{Block, BlockHash};
use ic_cdk::call::RejectCode;
use std::str::FromStr;
use std::{collections::HashMap, io::BufReader, path::PathBuf};
use std::{fs::File, panic::catch_unwind};

mod confirmation_counts;

/// Helper function to save a chain to a file in hex format.
#[cfg(feature = "save_chain_as_hex")]
fn save_chain_as_hex_file(chain: &[BitcoinBlock], file_name: &str) -> std::io::Result<()> {
    use std::io::{BufWriter, Write};
    let file = File::create(file_name)?;
    let mut writer = BufWriter::new(file);

    chain.iter().try_for_each(|block| {
        let mut bytes = Vec::new();
        block.consensus_encode(&mut bytes)?;
        writeln!(writer, "{}", hex::encode(bytes))
    })?;

    Ok(())
}

async fn process_chain(network: Network, blocks_file: &str, num_blocks: u32) {
    let mut chain: Vec<BitcoinBlock> = vec![];

    let mut blocks: HashMap<BlockHash, BitcoinBlock> = HashMap::new();

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
                Magic::from_bytes(magic.to_le_bytes())
            }
        };

        assert_eq!(
            magic,
            match network {
                Network::Mainnet => BitcoinNetwork::Bitcoin,
                Network::Testnet => BitcoinNetwork::Testnet4,
                Network::Regtest => BitcoinNetwork::Regtest,
            }
            .magic()
        );

        let _block_size = blk_file.read_u32::<LittleEndian>().unwrap();

        let block = BitcoinBlock::consensus_decode(&mut blk_file).unwrap();

        blocks.insert(BlockHash::from(block.header.prev_blockhash), block);
    }

    println!("# blocks in file: {}", blocks.len());

    // Build the chain
    chain.push(blocks.remove(genesis_block(network).block_hash()).unwrap());
    for _ in 1..num_blocks {
        let next_block = blocks
            .remove(&chain[chain.len() - 1].block_hash().into())
            .unwrap();
        chain.push(next_block);
    }

    println!("Built chain with length: {}", chain.len());

    #[cfg(feature = "save_chain_as_hex")]
    if network == Network::Testnet {
        save_chain_as_hex_file(
            &chain[..4000.min(chain.len())],
            "../benchmarks/src/testnet_blocks.txt",
        )
        .unwrap();
    }

    // Map the blocks into responses that are given to the hearbeat.
    let responses: Vec<_> = chain
        .into_iter()
        .map(|block| {
            let mut block_bytes = vec![];
            BitcoinBlock::consensus_encode(&block, &mut block_bytes).unwrap();
            GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
                GetSuccessorsCompleteResponse {
                    blocks: vec![block_bytes],
                    next: vec![],
                },
            ))
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

fn verify_block_header(state: &crate::State, height: u32, block_hash: &str) {
    let header = state.stable_block_headers.get_with_height(height).unwrap();
    let hash = header.block_hash().to_string();
    assert_eq!(block_hash, hash, "Block hash mismatch at height {}", height);

    let block_hash = BlockHash::from_str(block_hash).unwrap();
    let header_2 = state
        .stable_block_headers
        .get_with_block_hash(&block_hash)
        .unwrap();
    assert_eq!(header, header_2);
}

#[async_std::test]
async fn mainnet_100k_blocks() {
    crate::init(crate::InitConfig {
        stability_threshold: Some(10),
        network: Some(Network::Mainnet),
        ..Default::default()
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

    crate::with_state(|state| {
        let total_supply = state.utxos.get_total_supply();

        // NOTE: The duplicate transactions cause us to lose some of the supply,
        // which we deduct in this assertion.
        assert_eq!(
            ((state.utxos.next_height() as u64) - DUPLICATE_TX_IDS.len() as u64) * 5000000000,
            total_supply
        );
    });

    // Check some random addresses that the balance is correct:

    // https://blockexplorer.one/bitcoin/mainnet/address/1PgZsaGjvssNCqHHisshLoCFeUjxPhutTh
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1PgZsaGjvssNCqHHisshLoCFeUjxPhutTh".to_string(),
            min_confirmations: None
        })
        .unwrap(),
        4000000
    );

    assert_eq!(
        get_utxos(GetUtxosRequest {
            address: "1PgZsaGjvssNCqHHisshLoCFeUjxPhutTh".to_string(),
            filter: None
        })
        .unwrap(),
        GetUtxosResponse {
            utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: Txid::from_str(
                        "1a592a31c79f817ed787b6acbeef29b0f0324179820949d7da6215f0f4870c42",
                    )
                    .unwrap(),
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
        })
        .unwrap(),
        500000000
    );

    assert_eq!(
        get_utxos(GetUtxosRequest {
            address: "12tGGuawKdkw5NeDEzS3UANhCRa1XggBbK".to_string(),
            filter: None
        })
        .unwrap(),
        GetUtxosResponse {
            utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: Txid::from_str(
                        "3371b3978e7285d962fd54656aca6b3191135a1db838b5c689b8a44a7ede6a31",
                    )
                    .unwrap(),
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
        })
        .unwrap(),
        0
    );

    // At 10 confirmations it should have its BTC.
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro".to_string(),
            min_confirmations: Some(10)
        })
        .unwrap(),
        48_0000_0000
    );

    // At 6 confirmations it should have its BTC.
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro".to_string(),
            min_confirmations: Some(6)
        })
        .unwrap(),
        48_0000_0000
    );

    assert_eq!(
        get_utxos(GetUtxosRequest {
            address: "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro".to_string(),
            filter: Some(UtxosFilter::MinConfirmations(6))
        })
        .unwrap(),
        GetUtxosResponse {
            utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: Txid::from_str(
                        "2bdd8506980479fb57d848ddbbb29831b4d468f9dc5d572ccdea69edec677ed6",
                    )
                    .unwrap(),
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
        })
        .unwrap(),
        0
    );

    // The BTC is spent to the following two addresses.
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1NhzJ8bsdmGK39vSJtdQw3R2HyNtUmGxcr".to_string(),
            min_confirmations: Some(5),
        })
        .unwrap(),
        3_4500_0000
    );

    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "13U77vKQcTjpZ7gww4K8Nreq2ffGBQKxmr".to_string(),
            min_confirmations: Some(5)
        })
        .unwrap(),
        44_5500_0000
    );

    // And these addresses should have a balance of zero before that height.
    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "1NhzJ8bsdmGK39vSJtdQw3R2HyNtUmGxcr".to_string(),
            min_confirmations: Some(6),
        })
        .unwrap(),
        0
    );

    assert_eq!(
        get_balance(GetBalanceRequest {
            address: "13U77vKQcTjpZ7gww4K8Nreq2ffGBQKxmr".to_string(),
            min_confirmations: Some(6),
        })
        .unwrap(),
        0
    );

    // Check the block headers/heights of a few random blocks.
    crate::with_state(|state| {
        verify_block_header(
            state,
            0,
            &genesis_block(Network::Mainnet).block_hash().to_string(),
        );
        verify_block_header(
            state,
            14927,
            "000000005d8210ad23a745aac32e1a5aeb22e597df906c1f05cd642a87a672fa",
        );
        verify_block_header(
            state,
            99989,
            "000000000003e533769852c7373b155e898bbb6322c326c9a9ce3121f4fd5fd6",
        );
    });
}

#[async_std::test]
async fn testnet_10k_blocks() {
    crate::init(crate::InitConfig {
        stability_threshold: Some(2),
        network: Some(Network::Testnet),
        ..Default::default()
    });

    // Set a reasonable performance counter step to trigger time-slicing.
    runtime::set_performance_counter_step(100_000);

    process_chain(
        Network::Testnet,
        "test-data/testnet4_10k_blocks.dat",
        10_000,
    )
    .await;

    // Validate we've ingested all the blocks.
    assert_eq!(with_state(main_chain_height), 10_000);

    // Verify the total supply
    crate::with_state(|state| {
        let total_supply = state.utxos.get_total_supply();
        assert_eq!(state.utxos.next_height() as u64 * 5000000000, total_supply);
    });

    // Check the block headers/heights of a few random blocks.
    crate::with_state(|state| {
        // https://mempool.space/testnet4/block/00000000da84f2bafbbc53dee25a72ae507ff4914b867c565be350b0da8bf043
        verify_block_header(
            state,
            0,
            &genesis_block(Network::Testnet).block_hash().to_string(),
        );
        // https://mempool.space/testnet4/block/000000004deda718e1471a0b5899303e84df0d7a437284b93d29698724f11a0c
        verify_block_header(
            state,
            10,
            "000000004deda718e1471a0b5899303e84df0d7a437284b93d29698724f11a0c",
        );
        // https://mempool.space/testnet4/block/000000000286736136f91cad37d93209b204eb26ac5df3908a5695d8c38b2ffd
        verify_block_header(
            state,
            7182,
            "000000000286736136f91cad37d93209b204eb26ac5df3908a5695d8c38b2ffd",
        );
        // https://mempool.space/testnet4/block/000000000033c3815a71dde90eb10608a83fcef1f8448ce2e4de9a91a457350f
        verify_block_header(
            state,
            9997,
            "000000000033c3815a71dde90eb10608a83fcef1f8448ce2e4de9a91a457350f",
        );
    });
}

#[async_std::test]
async fn time_slices_large_block_with_multiple_transactions() {
    let network = Network::Regtest;
    let btc_network = into_bitcoin_network(network);
    init(InitConfig {
        stability_threshold: Some(0),
        network: Some(network),
        ..Default::default()
    });

    let address_1 = random_p2pkh_address(btc_network).into();
    let address_2 = random_p2pkh_address(btc_network).into();

    let tx_1 = TransactionBuilder::coinbase()
        .with_output(&address_1, 1000)
        .with_output(&address_1, 1000)
        .build();

    let tx_2 = TransactionBuilder::new()
        .with_output(&address_2, 1000)
        .with_output(&address_2, 1000)
        .build();

    let block_1 = BlockBuilder::with_prev_header(genesis_block(network).header())
        .with_transaction(tx_1)
        .with_transaction(tx_2)
        .build();

    // An additional block so that the previous block is ingested into the stable UTXO set.
    let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();

    // Serialize the blocks.
    let blocks: Vec<BlockBlob> = [block_1.clone(), block_2.clone()]
        .iter()
        .map(|block| {
            let mut block_bytes = vec![];
            block.consensus_encode(&mut block_bytes).unwrap();
            block_bytes
        })
        .collect();

    runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
        GetSuccessorsCompleteResponse {
            blocks,
            next: vec![],
        },
    )));

    // Set a large step for the performance_counter to exceed the instructions limit quickly.
    // This value allows ingesting 2 transactions inputs/outputs per round.
    runtime::set_performance_counter_step(375_000_000);

    // Fetch blocks.
    heartbeat().await;

    // Process response.
    heartbeat().await;

    // Assert that the block has been ingested.
    assert_eq!(with_state(main_chain_height), 2);

    // Run the heartbeat a few rounds to ingest the blocks.
    let expected_states = vec![
        IngestingBlock::new_with_args(block_1.clone(), 0, 1, 1),
        IngestingBlock::new_with_args(block_1.clone(), 1, 1, 1),
    ];

    for expected_state in expected_states.into_iter() {
        // Ingest stable blocks.
        runtime::performance_counter_reset();
        heartbeat().await;

        // Assert that execution has been paused.
        let partial_block = with_state(|s| s.utxos.ingesting_block.clone().unwrap());
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
    assert_eq!(with_state(|s| s.utxos.next_height()), 2);

    // Query the balance, expecting address 1 to be empty and address 2 to be non-empty.
    assert_eq!(
        get_balance(crate::types::GetBalanceRequest {
            address: address_1.to_string(),
            min_confirmations: None
        })
        .unwrap(),
        2000
    );

    assert_eq!(
        get_balance(crate::types::GetBalanceRequest {
            address: address_2.to_string(),
            min_confirmations: None
        })
        .unwrap(),
        2000
    );
}

#[async_std::test]
async fn test_rejections_counting() {
    crate::init(InitConfig::default());

    let counter_prior = crate::with_state(|state| state.syncing_state.num_get_successors_rejects);

    runtime::set_successors_response(GetSuccessorsReply::Err(
        RejectCode::CanisterReject,
        String::from("Test verification error."),
    ));

    // Fetch blocks.
    heartbeat().await;

    let counter_after = crate::with_state(|state| state.syncing_state.num_get_successors_rejects);

    assert_eq!(counter_prior, counter_after - 1);
}

// Serialize header.
fn get_header_blob(header: &Header) -> BlockHeaderBlob {
    let mut header_buff = vec![];
    header.consensus_encode(&mut header_buff).unwrap();
    header_buff.into()
}

fn get_chain_with_n_block_and_header_blobs(
    previous_block: &Block,
    n: usize,
) -> (Vec<Block>, Vec<BlockHeaderBlob>) {
    let block_vec = BlockChainBuilder::fork(previous_block, n as u32).build();

    let mut blob_vec = vec![];
    for block in block_vec.iter() {
        blob_vec.push(get_header_blob(block.header()));
    }
    (block_vec, blob_vec)
}

#[async_std::test]
async fn get_blockchain_info_succeeds_when_not_synced() {
    // get_blockchain_info is for monitoring and must succeed even when the canister
    // is not fully synced (verify_synced would panic for other endpoints).
    let network = Network::Regtest;

    init(InitConfig {
        stability_threshold: Some(2),
        network: Some(network),
        ..Default::default()
    });

    let block_1 = BlockBuilder::with_prev_header(genesis_block(network).header()).build();
    let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();

    let blocks: Vec<BlockBlob> = [block_1.clone(), block_2.clone()]
        .iter()
        .map(|block| {
            let mut block_bytes = vec![];
            block.consensus_encode(&mut block_bytes).unwrap();
            block_bytes
        })
        .collect();

    let (_, next_blocks_blobs) =
        get_chain_with_n_block_and_header_blobs(&block_2, (SYNCED_THRESHOLD + 1) as usize);

    runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
        GetSuccessorsCompleteResponse {
            blocks,
            next: next_blocks_blobs,
        },
    )));

    heartbeat().await;
    heartbeat().await;
    heartbeat().await;

    assert_eq!(with_state(main_chain_height), 2);
    assert!(
        with_state(|s| s.unstable_blocks.next_block_headers_max_height().unwrap())
            > with_state(main_chain_height) + SYNCED_THRESHOLD
    );
    assert!(catch_unwind(verify_synced).is_err());

    let info = get_blockchain_info();
    assert_eq!(info.height, 2);
}

#[async_std::test]
async fn test_syncing_with_next_block_headers() {
    let network = Network::Regtest;

    init(InitConfig {
        stability_threshold: Some(2),
        network: Some(network),
        ..Default::default()
    });

    let block_1 = BlockBuilder::with_prev_header(genesis_block(network).header()).build();

    let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();

    // Serialize the blocks.
    let blocks: Vec<BlockBlob> = [block_1.clone(), block_2.clone()]
        .iter()
        .map(|block| {
            let mut block_bytes = vec![];
            block.consensus_encode(&mut block_bytes).unwrap();
            block_bytes
        })
        .collect();

    let (next_blocks, next_blocks_blobs) =
        get_chain_with_n_block_and_header_blobs(&block_2, (SYNCED_THRESHOLD + 1) as usize);
    // We now have a chain of SYNCED_THRESHOLD + 1 next blocks
    // extending the unstable block (block_2).
    runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
        GetSuccessorsCompleteResponse {
            blocks,
            next: next_blocks_blobs,
        },
    )));

    // Fetch blocks.
    heartbeat().await;

    // Process response.
    heartbeat().await;

    // Ingest StableBlocks (block_1) into the UTXO set.
    heartbeat().await;

    // Assert that the block has been ingested.
    assert_eq!(with_state(main_chain_height), 2);

    assert_eq!(with_state(|s| s.stable_height()), 1);

    assert_eq!(
        with_state(|s| s.unstable_blocks.next_block_headers_max_height().unwrap()),
        with_state(main_chain_height) + SYNCED_THRESHOLD + 1
    );

    assert!(catch_unwind(verify_synced).is_err());

    let mut first_next_block_bytes = vec![];

    next_blocks[0]
        .clone()
        .consensus_encode(&mut first_next_block_bytes)
        .unwrap();

    // We now have 2 UnstableBlocks and chain of SYNCED_THRESHOLD next blocks
    // extending the last unstable block(first_next_block).
    runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
        GetSuccessorsCompleteResponse {
            blocks: vec![first_next_block_bytes],
            next: vec![],
        },
    )));

    // Fetch blocks.
    heartbeat().await;

    // Process response.
    heartbeat().await;

    // Ingest StableBlocks (block_2) into the UTXO set.
    heartbeat().await;

    // Assert that the block has been ingested.
    assert_eq!(with_state(main_chain_height), 3);

    assert_eq!(with_state(|s| s.stable_height()), 2);

    assert_eq!(
        with_state(|s| s.unstable_blocks.next_block_headers_max_height().unwrap()),
        with_state(main_chain_height) + SYNCED_THRESHOLD
    );

    verify_synced();

    let (next_blocks, next_blocks_blobs) =
        get_chain_with_n_block_and_header_blobs(&block_2, (SYNCED_THRESHOLD + 1) as usize);

    // We now have 1 UnstableBlocks and chain of SYNCED_THRESHOLD + 2 next blocks
    // extending the last stable block (block_1). Hence it is SYNCED_THRESHOLD + 1
    // longer than main_chain.
    runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
        GetSuccessorsCompleteResponse {
            blocks: vec![],
            next: next_blocks_blobs,
        },
    )));

    // Fetch blocks.
    heartbeat().await;

    // Process response.
    heartbeat().await;

    // Try to ingest StableBlocks into the UTXO set.
    heartbeat().await;

    // Assert that the block has been ingested.
    assert_eq!(with_state(main_chain_height), 3);

    assert_eq!(with_state(|s| s.stable_height()), 2);

    assert_eq!(
        with_state(|s| s.unstable_blocks.next_block_headers_max_height().unwrap()),
        with_state(main_chain_height) + SYNCED_THRESHOLD
    );

    verify_synced();

    // We are extending the longest chain of next blocks.
    runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
        GetSuccessorsCompleteResponse {
            blocks: vec![],
            next: get_chain_with_n_block_and_header_blobs(next_blocks.last().unwrap(), 1).1,
        },
    )));

    // Fetch blocks.
    heartbeat().await;

    // Process response.
    heartbeat().await;

    // Try to ingest StableBlocks into the UTXO set.
    heartbeat().await;

    // Assert that the block has been ingested.
    assert_eq!(with_state(main_chain_height), 3);

    assert_eq!(with_state(|s| s.stable_height()), 2);

    assert_eq!(
        with_state(|s| s.unstable_blocks.next_block_headers_max_height().unwrap()),
        with_state(main_chain_height) + SYNCED_THRESHOLD + 1
    );

    assert!(catch_unwind(verify_synced).is_err());
}

#[async_std::test]
async fn cycles_burnt_are_tracked_in_metrics() {
    crate::init(InitConfig {
        burn_cycles: Some(Flag::Enabled),
        ..Default::default()
    });

    let cycles_burnt_0 = crate::with_state(|state| state.metrics.cycles_burnt);

    assert_eq!(cycles_burnt_0, Some(0));

    let burn_amount = 1_000_000;

    // Burn cycles.
    heartbeat().await;

    let cycles_burnt_1 = crate::with_state(|state| state.metrics.cycles_burnt);

    assert_eq!(cycles_burnt_1, Some(burn_amount));

    // Burn cycles.
    heartbeat().await;

    let cycles_burnt_2 = crate::with_state(|state| state.metrics.cycles_burnt);

    assert_eq!(cycles_burnt_2, Some(2 * burn_amount));

    // Burn cycles.
    heartbeat().await;

    let cycles_burnt_3 = crate::with_state(|state| state.metrics.cycles_burnt);

    assert_eq!(cycles_burnt_3, Some(3 * burn_amount));
}

#[async_std::test]
async fn cycles_are_not_burnt_when_flag_is_disabled() {
    crate::init(InitConfig {
        burn_cycles: Some(Flag::Disabled),
        ..Default::default()
    });

    assert_eq!(
        crate::with_state(|state| state.metrics.cycles_burnt),
        Some(0)
    );

    // Run the heartbeat.
    heartbeat().await;

    // No cycles should be burnt.
    assert_eq!(
        crate::with_state(|state| state.metrics.cycles_burnt),
        Some(0)
    );
}

async fn fee_percentiles_evaluation_helper() {
    // Create a block with a transaction that has fees.
    let block_0 = {
        let fee = 1;
        let balance = 1000;
        let network = Network::Regtest;
        let btc_network = into_bitcoin_network(network);

        let tx_1 = TransactionBuilder::coinbase()
            .with_output(&random_p2pkh_address(btc_network).into(), balance)
            .build();
        let tx_2 = TransactionBuilder::new()
            .with_input(ic_btc_types::OutPoint {
                txid: tx_1.txid(),
                vout: 0,
            })
            .with_output(&random_p2pkh_address(btc_network).into(), balance - fee)
            .build();

        BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(tx_1)
            .with_transaction(tx_2.clone())
            .build()
    };

    let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();

    // Serialize the block.
    let blocks: Vec<BlockBlob> = [block_0.clone(), block_1.clone()]
        .iter()
        .map(|block| {
            let mut block_bytes = vec![];
            block.consensus_encode(&mut block_bytes).unwrap();
            block_bytes
        })
        .collect();

    runtime::set_successors_response(GetSuccessorsReply::Ok(GetSuccessorsResponse::Complete(
        GetSuccessorsCompleteResponse {
            blocks,
            next: vec![],
        },
    )));

    // Run the heartbeat to fetch the blocks.
    heartbeat().await;

    // Run the heartbeat to ingest the blocks.
    heartbeat().await;

    // Verify the blocks have been ingested.
    assert_eq!(with_state(main_chain_height), 2);

    // New blocks are not yet marked as stable.
    assert_eq!(with_state(|s| s.stable_height()), 0);

    // Run the heartbeat for blocks to be marked as stable.
    heartbeat().await;

    // New blocks are now marked as stable.
    assert_eq!(with_state(|s| s.stable_height()), 2);
}

#[async_std::test]
async fn fee_percentiles_are_evaluated_lazily() {
    crate::init(InitConfig {
        lazily_evaluate_fee_percentiles: Some(Flag::Enabled),
        stability_threshold: Some(0),
        ..Default::default()
    });

    fee_percentiles_evaluation_helper().await;

    // Fee percentiles should be empty, since there are no transactions
    // in the unstable blocks.
    assert_eq!(get_current_fee_percentiles().len(), 0);
}

#[async_std::test]
async fn fee_percentiles_are_evaluated_eagerly() {
    crate::init(InitConfig {
        lazily_evaluate_fee_percentiles: Some(Flag::Disabled),
        stability_threshold: Some(0),
        ..Default::default()
    });

    fee_percentiles_evaluation_helper().await;

    // Even though there are no transactions in the unstable blocks, fee
    // percentiles should NOT be empty, as they were eagerly evaluated
    // when blocks were ingested.
    assert_eq!(get_current_fee_percentiles().len(), 101);
}
