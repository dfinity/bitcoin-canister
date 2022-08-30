use crate::{
    blocktree::{BlockChain, BlockDoesNotExtendTree},
    runtime::performance_counter,
    state::{PartialStableBlock, State},
    types::{OutPoint, Page},
    unstable_blocks, utxoset,
};
use bitcoin::{Address, Block, Txid};
use ic_btc_types::{GetBalanceError, GetUtxosError, GetUtxosResponse, Height, Satoshi};
use lazy_static::lazy_static;
use serde_bytes::ByteBuf;
use std::str::FromStr;

lazy_static! {
    static ref DUPLICATE_TX_IDS: [Txid; 2] = [
        Txid::from_str("d5d27987d2a3dfc724e359870c6644b40e497bdc0589a033220fe15429d88599").unwrap(),
        Txid::from_str("e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468").unwrap()
    ];
}

// The threshold at which time slicing kicks in.
// At the time of this writing it is equivalent to 80% of the maximum instructions limit.
const MAX_INSTRUCTIONS_THRESHOLD: u64 = 4_000_000_000;

/// Returns the balance of a bitcoin address.
// TODO(EXC-1203): Move this method into api/get_balance.rs
pub fn get_balance(
    state: &State,
    address: &str,
    min_confirmations: u32,
) -> Result<Satoshi, GetBalanceError> {
    // NOTE: It is safe to sum up the balances here without the risk of overflow.
    // The maximum number of bitcoins is 2.1 * 10^7, which is 2.1* 10^15 satoshis.
    // That is well below the max value of a `u64`.
    let mut balance = 0;
    match get_utxos(state, address, min_confirmations, None, None) {
        Ok(res) => {
            for utxo in res.utxos {
                balance += utxo.value;
            }

            Ok(balance)
        }
        Err(err) => match err {
            GetUtxosError::MalformedAddress => Err(GetBalanceError::MalformedAddress),
            GetUtxosError::MinConfirmationsTooLarge { given, max } => {
                Err(GetBalanceError::MinConfirmationsTooLarge { given, max })
            }
            err => unreachable!("Got unexpected error: {}", err),
        },
    }
}

/// Returns the set of UTXOs for a given bitcoin address.
///
/// Transactions with confirmations < `min_confirmations` are not considered.
///
/// If the optional `page` is set, then it will be used to return the next chunk
/// of UTXOs starting from that page reference.
///
/// The optional `utxo_limit` restricts the number of UTXOs that can be included
/// in the response in case there are too many UTXOs for this address and they
/// cannot fit in a single response. A `page` reference will be returned along
/// the list of UTXOs in this case that can be used in a subsequent request
/// to retrieve the remaining UTXOs.
pub fn get_utxos(
    state: &State,
    address: &str,
    min_confirmations: u32,
    page: Option<Vec<u8>>,
    utxo_limit: Option<usize>,
) -> Result<GetUtxosResponse, GetUtxosError> {
    match page {
        // A page was provided in the request, so we should use it as a basis
        // to compute the next chunk of UTXOs to be returned.
        Some(page) => {
            let Page {
                tip_block_hash,
                height,
                outpoint,
            } = Page::from_bytes(page).map_err(|err| GetUtxosError::MalformedPage { err })?;
            let chain =
                unstable_blocks::get_chain_with_tip(&state.unstable_blocks, &tip_block_hash)
                    .ok_or(GetUtxosError::UnknownTipBlockHash {
                        tip_block_hash: tip_block_hash.to_vec(),
                    })?;
            get_utxos_from_chain(
                state,
                address,
                min_confirmations,
                chain,
                Some((height, outpoint)),
                utxo_limit,
            )
        }
        // No specific page was provided, so we use the main chain for computing UTXOs.
        None => {
            let chain = unstable_blocks::get_main_chain(&state.unstable_blocks);
            get_utxos_from_chain(state, address, min_confirmations, chain, None, utxo_limit)
        }
    }
}

fn get_utxos_from_chain(
    state: &State,
    address: &str,
    min_confirmations: u32,
    chain: BlockChain,
    offset: Option<(Height, OutPoint)>,
    utxo_limit: Option<usize>,
) -> Result<GetUtxosResponse, GetUtxosError> {
    if Address::from_str(address).is_err() {
        return Err(GetUtxosError::MalformedAddress);
    }

    if chain.len() < min_confirmations as usize {
        return Err(GetUtxosError::MinConfirmationsTooLarge {
            given: min_confirmations,
            max: chain.len() as u32,
        });
    }

    let mut address_utxos = utxoset::get_utxos(&state.utxos, address);
    let chain_height = state.height + (chain.len() as u32) - 1;

    let mut tip_block_hash = chain.first().block_hash();
    let mut tip_block_height = state.height;

    // Apply unstable blocks to the UTXO set.
    for (i, block) in chain.into_chain().iter().enumerate() {
        let block_height = state.height + (i as u32);
        let confirmations = chain_height - block_height + 1;

        if confirmations < min_confirmations {
            // The block has fewer confirmations than requested.
            // We can stop now since all remaining blocks will have fewer confirmations.
            break;
        }

        for tx in &block.txdata {
            address_utxos.insert_tx(tx, block_height);
        }

        tip_block_hash = block.block_hash();
        tip_block_height = block_height;
    }

    let all_utxos = address_utxos.into_vec(offset);
    let mut next_page = None;

    let utxos = match utxo_limit {
        // No specific limit set, we should return all utxos.
        None => all_utxos,
        // There's some limit, so use it to chunk up the UTXOs if they don't fit.
        Some(utxo_limit) => {
            let (utxos_to_return, rest) =
                all_utxos.split_at(all_utxos.len().min(utxo_limit as usize));

            if !rest.is_empty() {
                next_page = Some(
                    Page {
                        tip_block_hash,
                        height: rest[0].height,
                        outpoint: OutPoint::new(
                            rest[0].outpoint.txid.clone(),
                            rest[0].outpoint.vout,
                        ),
                    }
                    .to_bytes(),
                );
            }

            utxos_to_return.to_vec()
        }
    };

    Ok(GetUtxosResponse {
        utxos,
        tip_block_hash: tip_block_hash.to_vec(),
        tip_height: tip_block_height,
        next_page: next_page.map(ByteBuf::from),
    })
}

/// Inserts a block into the state.
/// Returns an error if the block doesn't extend any known block in the state.
pub fn insert_block(state: &mut State, block: Block) -> Result<(), BlockDoesNotExtendTree> {
    unstable_blocks::push(&mut state.unstable_blocks, block)
}

/// Pops any blocks in `UnstableBlocks` that are considered stable and ingests them to the UTXO set.
///
/// NOTE: This method does a form of time-slicing to stay within the instruction limit, and
/// multiple calls may be required for all the stable blocks to be written.
///
/// Returns a bool indicating whether or not new transactions have been inserted into the UTXO set.
pub fn ingest_stable_blocks_into_utxoset(state: &mut State) -> bool {
    enum Slicing {
        Paused,
        Done,
    }

    let mut has_inserted_txs = false;

    // A closure for writing a block into the UTXO set, inserting as many transactions as possible
    // within the instructions limit.
    let mut ingest_block_into_utxoset =
        |state: &mut State, block: Block, txs_to_skip: usize| -> Slicing {
            for (tx_idx, tx) in block.txdata.iter().enumerate().skip(txs_to_skip) {
                if performance_counter() > MAX_INSTRUCTIONS_THRESHOLD {
                    // Getting close the the instructions limit. Pause execution.
                    state.syncing_state.partial_stable_block = Some(PartialStableBlock {
                        block,
                        txs_processed: tx_idx,
                    });

                    return Slicing::Paused;
                }

                utxoset::insert_tx(&mut state.utxos, tx, state.height);
                has_inserted_txs = true;
            }

            state.height += 1;

            Slicing::Done
        };

    // Finish writing the stable block that's partially written, if that exists.
    if let Some(partial_stable_block) = state.syncing_state.partial_stable_block.take() {
        match ingest_block_into_utxoset(
            state,
            partial_stable_block.block,
            partial_stable_block.txs_processed,
        ) {
            Slicing::Paused => return has_inserted_txs,
            Slicing::Done => {}
        }
    }

    // Check if there are any stable blocks and ingest those into the UTXO set.
    while let Some(new_stable_block) = unstable_blocks::pop(&mut state.unstable_blocks) {
        match ingest_block_into_utxoset(state, new_stable_block, 0) {
            Slicing::Paused => return has_inserted_txs,
            Slicing::Done => {}
        }
    }

    has_inserted_txs
}

pub fn main_chain_height(state: &State) -> Height {
    unstable_blocks::get_main_chain(&state.unstable_blocks).len() as u32 + state.height - 1
}

pub fn get_unstable_blocks(state: &State) -> Vec<&Block> {
    unstable_blocks::get_blocks(&state.unstable_blocks)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::random_p2pkh_address;
    use crate::types::Network;
    use bitcoin::blockdata::constants::genesis_block;
    use bitcoin::secp256k1::rand::rngs::OsRng;
    use bitcoin::secp256k1::Secp256k1;
    use bitcoin::{consensus::Decodable, Address, BlockHash, Network as BitcoinNetwork, PublicKey};
    use byteorder::{LittleEndian, ReadBytesExt};
    use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};
    use ic_btc_types::{OutPoint, Utxo};
    use proptest::prelude::*;
    use std::fs::File;
    use std::str::FromStr;
    use std::{collections::HashMap, io::BufReader, path::PathBuf};

    fn process_chain(state: &mut State, num_blocks: u32) {
        let mut chain: Vec<Block> = vec![];

        let mut blocks: HashMap<BlockHash, Block> = HashMap::new();

        let mut blk_file = BufReader::new(
            File::open(
                PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                    .join("test-data/100k_blocks.dat"),
            )
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
            assert_eq!(magic, 0xD9B4BEF9);

            let _block_size = blk_file.read_u32::<LittleEndian>().unwrap();

            let block = Block::consensus_decode(&mut blk_file).unwrap();

            blocks.insert(block.header.prev_blockhash, block);
        }

        println!("# blocks in file: {}", blocks.len());

        // Build the chain
        chain.push(
            blocks
                .remove(&genesis_block(BitcoinNetwork::Bitcoin).block_hash())
                .unwrap(),
        );
        for _ in 1..num_blocks {
            let next_block = blocks.remove(&chain[chain.len() - 1].block_hash()).unwrap();
            chain.push(next_block);
        }

        println!("Built chain with length: {}", chain.len());

        let mut i = 0;
        for block in chain.into_iter() {
            insert_block(state, block).unwrap();
            ingest_stable_blocks_into_utxoset(state);
            i += 1;
            if i % 1000 == 0 {
                println!("processed block: {}", i);
            }
        }
    }

    /*
    #[test]
    fn to_from_proto() {
        let root: PathBuf = tempfile::Builder::new()
            .prefix("bitcoin")
            .tempdir()
            .unwrap()
            .path()
            .into();

        let mut block = BlockBuilder::genesis()
            .with_transaction(TransactionBuilder::coinbase().build())
            .build();
        let mut state = State::new(2, Network::Bitcoin, block.clone());

        for _ in 0..100 {
            block = BlockBuilder::with_prev_header(block.header)
                .with_transaction(TransactionBuilder::coinbase().build())
                .build();
            insert_block(&mut state, block.clone()).unwrap();
        }

        state.serialize(&root).unwrap();

        let new_state = State::load(&root).unwrap();

        assert_eq!(new_state.height, state.height);
        assert_eq!(new_state.unstable_blocks, state.unstable_blocks);
        assert_eq!(new_state.utxos.network, state.utxos.network);
        assert_eq!(
            new_state.utxos.utxos.large_utxos,
            state.utxos.utxos.large_utxos
        );

        for (new_entry, old_entry) in new_state.utxos.utxos.iter().zip(state.utxos.utxos.iter()) {
            assert_eq!(new_entry, old_entry);
        }

        assert_eq!(
            new_state.utxos.address_to_outpoints.len(),
            state.utxos.address_to_outpoints.len()
        );

        for (new_entry, old_entry) in new_state
            .utxos
            .address_to_outpoints
            .iter()
            .zip(state.utxos.address_to_outpoints.iter())
        {
            assert_eq!(new_entry, old_entry);
        }
    }*/

    #[test]
    fn utxos_forks() {
        let secp = Secp256k1::new();
        let mut rng = OsRng::new().unwrap();

        // Create some BTC addresses.
        let address_1 = Address::p2pkh(
            &PublicKey::new(secp.generate_keypair(&mut rng).1),
            BitcoinNetwork::Bitcoin,
        );
        let address_2 = Address::p2pkh(
            &PublicKey::new(secp.generate_keypair(&mut rng).1),
            BitcoinNetwork::Bitcoin,
        );
        let address_3 = Address::p2pkh(
            &PublicKey::new(secp.generate_keypair(&mut rng).1),
            BitcoinNetwork::Bitcoin,
        );
        let address_4 = Address::p2pkh(
            &PublicKey::new(secp.generate_keypair(&mut rng).1),
            BitcoinNetwork::Bitcoin,
        );

        // Create a genesis block where 1000 satoshis are given to address 1.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();

        let block_0 = BlockBuilder::genesis()
            .with_transaction(coinbase_tx.clone())
            .build();

        let mut state = State::new(2, Network::Mainnet, block_0.clone());

        let block_0_utxos = GetUtxosResponse {
            utxos: vec![Utxo {
                outpoint: OutPoint {
                    txid: coinbase_tx.txid().to_vec(),
                    vout: 0,
                },
                value: 1000,
                height: 0,
            }],
            tip_block_hash: block_0.block_hash().to_vec(),
            tip_height: 0,
            next_page: None,
        };

        // Assert that the UTXOs of address 1 are present.
        assert_eq!(
            get_utxos(&state, &address_1.to_string(), 0, None, None),
            Ok(block_0_utxos.clone())
        );

        // Extend block 0 with block 1 that spends the 1000 satoshis and gives them to address 2.
        let tx = TransactionBuilder::new()
            .with_input(bitcoin::OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_2, 1000)
            .build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header)
            .with_transaction(tx.clone())
            .build();

        insert_block(&mut state, block_1.clone()).unwrap();

        // address 2 should now have the UTXO while address 1 has no UTXOs.
        assert_eq!(
            get_utxos(&state, &address_2.to_string(), 0, None, None),
            Ok(GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: tx.txid().to_vec(),
                        vout: 0,
                    },
                    value: 1000,
                    height: 1,
                }],
                tip_block_hash: block_1.block_hash().to_vec(),
                tip_height: 1,
                next_page: None,
            })
        );

        assert_eq!(
            get_utxos(&state, &address_1.to_string(), 0, None, None),
            Ok(GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_1.block_hash().to_vec(),
                tip_height: 1,
                next_page: None,
            })
        );

        // Extend block 0 (again) with block 1 that spends the 1000 satoshis to address 3
        // This causes a fork.
        let tx = TransactionBuilder::new()
            .with_input(bitcoin::OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_3, 1000)
            .build();
        let block_1_prime = BlockBuilder::with_prev_header(block_0.header)
            .with_transaction(tx.clone())
            .build();
        insert_block(&mut state, block_1_prime.clone()).unwrap();

        // Because block 1 and block 1' contest with each other, neither of them are included
        // in the UTXOs. Only the UTXOs of block 0 are returned.
        assert_eq!(
            get_utxos(&state, &address_2.to_string(), 0, None, None),
            Ok(GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_0.block_hash().to_vec(),
                tip_height: 0,
                next_page: None,
            })
        );
        assert_eq!(
            get_utxos(&state, &address_3.to_string(), 0, None, None),
            Ok(GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_0.block_hash().to_vec(),
                tip_height: 0,
                next_page: None,
            })
        );
        assert_eq!(
            get_utxos(&state, &address_1.to_string(), 0, None, None),
            Ok(block_0_utxos)
        );

        // Now extend block 1' with another block that transfers the funds to address 4.
        // In this case, the fork of [block 1', block 2'] will be considered the "main"
        // chain, and will be part of the UTXOs.
        let tx = TransactionBuilder::new()
            .with_input(bitcoin::OutPoint::new(tx.txid(), 0))
            .with_output(&address_4, 1000)
            .build();
        let block_2_prime = BlockBuilder::with_prev_header(block_1_prime.header)
            .with_transaction(tx.clone())
            .build();
        insert_block(&mut state, block_2_prime.clone()).unwrap();

        // Address 1 has no UTXOs since they were spent on the main chain.
        assert_eq!(
            get_utxos(&state, &address_1.to_string(), 0, None, None),
            Ok(GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_2_prime.block_hash().to_vec(),
                tip_height: 2,
                next_page: None,
            })
        );
        assert_eq!(
            get_utxos(&state, &address_2.to_string(), 0, None, None),
            Ok(GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_2_prime.block_hash().to_vec(),
                tip_height: 2,
                next_page: None,
            })
        );
        assert_eq!(
            get_utxos(&state, &address_3.to_string(), 0, None, None),
            Ok(GetUtxosResponse {
                utxos: vec![],
                tip_block_hash: block_2_prime.block_hash().to_vec(),
                tip_height: 2,
                next_page: None,
            })
        );
        // The funds are now with address 4.
        assert_eq!(
            get_utxos(&state, &address_4.to_string(), 0, None, None),
            Ok(GetUtxosResponse {
                utxos: vec![Utxo {
                    outpoint: OutPoint {
                        txid: tx.txid().to_vec(),
                        vout: 0,
                    },
                    value: 1000,
                    height: 2,
                }],
                tip_block_hash: block_2_prime.block_hash().to_vec(),
                tip_height: 2,
                next_page: None,
            })
        );
    }

    #[test]
    fn process_100k_blocks() {
        let mut state = State::new(10, Network::Mainnet, genesis_block(BitcoinNetwork::Bitcoin));

        process_chain(&mut state, 100_000);

        let mut total_supply = 0;
        for (_, (v, _)) in state.utxos.utxos.iter() {
            total_supply += v.value;
        }

        // NOTE: The duplicate transactions cause us to lose some of the supply,
        // which we deduct in this assertion.
        assert_eq!(
            ((state.height as u64) - DUPLICATE_TX_IDS.len() as u64) * 5000000000,
            total_supply
        );

        // Check some random addresses that the balance is correct:

        // https://blockexplorer.one/bitcoin/mainnet/address/1PgZsaGjvssNCqHHisshLoCFeUjxPhutTh
        assert_eq!(
            get_balance(&state, "1PgZsaGjvssNCqHHisshLoCFeUjxPhutTh", 0),
            Ok(4000000)
        );

        assert_eq!(
            get_utxos(&state, "1PgZsaGjvssNCqHHisshLoCFeUjxPhutTh", 0, None, None),
            Ok(GetUtxosResponse {
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
            })
        );

        // https://blockexplorer.one/bitcoin/mainnet/address/12tGGuawKdkw5NeDEzS3UANhCRa1XggBbK
        assert_eq!(
            get_balance(&state, "12tGGuawKdkw5NeDEzS3UANhCRa1XggBbK", 0),
            Ok(500000000)
        );
        assert_eq!(
            get_utxos(&state, "12tGGuawKdkw5NeDEzS3UANhCRa1XggBbK", 0, None, None),
            Ok(GetUtxosResponse {
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
            })
        );

        // This address spent its BTC at height 99,996. At 0 confirmations
        // (height 100,000) it should have no BTC.
        assert_eq!(
            get_balance(&state, "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro", 0),
            Ok(0)
        );

        // At 10 confirmations it should have its BTC.
        assert_eq!(
            get_balance(&state, "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro", 10),
            Ok(48_0000_0000)
        );

        // At 6 confirmations it should have its BTC.
        assert_eq!(
            get_balance(&state, "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro", 6),
            Ok(48_0000_0000)
        );

        assert_eq!(
            get_utxos(&state, "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro", 6, None, None),
            Ok(GetUtxosResponse {
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
            })
        );

        // At 5 confirmations the BTC is spent.
        assert_eq!(
            get_balance(&state, "1K791w8Y1CXwyG3zAf9EzpoZvpYH8Z2Rro", 5),
            Ok(0)
        );

        // The BTC is spent to the following two addresses.
        assert_eq!(
            get_balance(&state, "1NhzJ8bsdmGK39vSJtdQw3R2HyNtUmGxcr", 5),
            Ok(3_4500_0000)
        );

        assert_eq!(
            get_balance(&state, "13U77vKQcTjpZ7gww4K8Nreq2ffGBQKxmr", 5),
            Ok(44_5500_0000)
        );

        // And these addresses should have a balance of zero before that height.
        assert_eq!(
            get_balance(&state, "1NhzJ8bsdmGK39vSJtdQw3R2HyNtUmGxcr", 6),
            Ok(0)
        );

        assert_eq!(
            get_balance(&state, "13U77vKQcTjpZ7gww4K8Nreq2ffGBQKxmr", 6),
            Ok(0)
        );
    }

    #[test]
    fn get_utxos_min_confirmations_greater_than_chain_height() {
        for network in [Network::Mainnet, Network::Testnet, Network::Regtest].iter() {
            // Generate addresses.
            let address_1 = random_p2pkh_address(*network);

            // Create a block where 1000 satoshis are given to the address_1.
            let tx = TransactionBuilder::coinbase()
                .with_output(&address_1, 1000)
                .build();
            let block_0 = BlockBuilder::genesis().with_transaction(tx.clone()).build();

            let state = State::new(1, *network, block_0.clone());

            // Expect an empty UTXO set.
            assert_eq!(
                get_utxos(&state, &address_1.to_string(), 1, None, None),
                Ok(GetUtxosResponse {
                    utxos: vec![Utxo {
                        outpoint: OutPoint {
                            txid: tx.txid().to_vec(),
                            vout: 0
                        },
                        value: 1000,
                        height: 0,
                    }],
                    tip_block_hash: block_0.block_hash().to_vec(),
                    tip_height: 0,
                    next_page: None,
                })
            );
            assert_eq!(
                get_utxos(&state, &address_1.to_string(), 2, None, None),
                Err(GetUtxosError::MinConfirmationsTooLarge { given: 2, max: 1 })
            );
        }
    }

    #[test]
    fn get_utxos_does_not_include_other_addresses() {
        for network in [Network::Mainnet, Network::Testnet, Network::Regtest].iter() {
            // Generate addresses.
            let address_1 = random_p2pkh_address(*network);

            let address_2 = random_p2pkh_address(*network);

            // Create a genesis block where 1000 satoshis are given to the address_1, followed
            // by a block where address_1 gives 1000 satoshis to address_2.
            let coinbase_tx = TransactionBuilder::coinbase()
                .with_output(&address_1, 1000)
                .build();
            let block_0 = BlockBuilder::genesis()
                .with_transaction(coinbase_tx.clone())
                .build();
            let tx = TransactionBuilder::new()
                .with_input(bitcoin::OutPoint::new(coinbase_tx.txid(), 0))
                .with_output(&address_2, 1000)
                .build();
            let block_1 = BlockBuilder::with_prev_header(block_0.header)
                .with_transaction(tx.clone())
                .build();

            let mut state = State::new(2, *network, block_0);
            insert_block(&mut state, block_1.clone()).unwrap();

            // Address 1 should have no UTXOs at zero confirmations.
            assert_eq!(
                get_utxos(&state, &address_1.to_string(), 0, None, None),
                Ok(GetUtxosResponse {
                    utxos: vec![],
                    tip_block_hash: block_1.block_hash().to_vec(),
                    tip_height: 1,
                    next_page: None,
                })
            );
        }
    }

    #[test]
    fn get_utxos_for_address_with_many_of_them_respects_utxo_limit() {
        for network in [Network::Mainnet, Network::Testnet, Network::Regtest].iter() {
            // Generate an address.
            let address = random_p2pkh_address(*network);

            let num_transactions = 10;
            let mut transactions = vec![];
            for i in 0..num_transactions {
                transactions.push(
                    TransactionBuilder::coinbase()
                        .with_output(&address, (i + 1) * 10)
                        .build(),
                );
            }

            let mut block_builder = BlockBuilder::genesis();
            for transaction in transactions.iter() {
                block_builder = block_builder.with_transaction(transaction.clone());
            }
            let block_0 = block_builder.build();
            let state = State::new(2, *network, block_0.clone());
            let tip_block_hash = block_0.block_hash();

            let utxo_set = get_utxos(&state, &address.to_string(), 0, None, None)
                .unwrap()
                .utxos;

            // Only some UTXOs can be included given that we use a utxo limit.
            let response = get_utxos(
                &state,
                &address.to_string(),
                0,
                None,
                // Allow 3 UTXOs to be returned.
                Some(3),
            )
            .unwrap();

            assert_eq!(response.utxos.len(), 3);
            assert!(response.utxos.len() < utxo_set.len());
            assert_eq!(response.tip_block_hash, tip_block_hash.clone().to_vec());
            assert_eq!(response.tip_height, 0);
            assert!(response.next_page.is_some());

            // A bigger limit allows more UTXOs to be included in a single response.
            let response = get_utxos(
                &state,
                &address.to_string(),
                0,
                None,
                // Allow 4 UTXOs to be returned.
                Some(4),
            )
            .unwrap();

            assert_eq!(response.utxos.len(), 4);
            assert!(response.utxos.len() < utxo_set.len());
            assert_eq!(response.tip_block_hash, tip_block_hash.clone().to_vec());
            assert_eq!(response.tip_height, 0);
            assert!(response.next_page.is_some());

            // A very big limit will result in the same as requesting UTXOs without any limit.
            let response = get_utxos(&state, &address.to_string(), 0, None, Some(1000)).unwrap();

            assert_eq!(response.utxos.len(), num_transactions as usize);
            assert_eq!(response.utxos.len(), utxo_set.len());
            assert_eq!(response.tip_block_hash, tip_block_hash.clone().to_vec());
            assert_eq!(response.tip_height, 0);
            assert!(response.next_page.is_none());
        }
    }

    proptest! {
        #[test]
        fn get_utxos_with_pagination_is_consistent_with_no_pagination(
            network in prop_oneof![
                Just(Network::Mainnet),
                Just(Network::Testnet),
                Just(Network::Regtest),
            ],
            num_transactions in 1..20u64,
            num_blocks in 1..10u64,
            utxo_limit in prop_oneof![
                Just(10),
                Just(20),
                Just(50),
                Just(100),
            ],
        ) {
            // Generate an address.
            let address = random_p2pkh_address(network);

            let mut prev_block: Option<Block> = None;
            let mut value = 1;
            let mut blocks = vec![];
            for block_idx in 0..num_blocks {
                let mut block_builder = match prev_block {
                    Some(b) => BlockBuilder::with_prev_header(b.header),
                    None => BlockBuilder::genesis(),
                };

                let mut transactions = vec![];
                for _ in 0..(num_transactions + block_idx) {
                    transactions.push(
                        TransactionBuilder::coinbase()
                            .with_output(&address, value)
                            .build()
                    );
                    // Vary the value of the transaction to ensure that
                    // we get unique outpoints in the blockchain.
                    value += 1;
                }

                for transaction in transactions.iter() {
                    block_builder = block_builder.with_transaction(transaction.clone());
                }

                let block = block_builder.build();
                blocks.push(block.clone());
                prev_block = Some(block);
            }

            let mut state = State::new(2, network, blocks[0].clone());
            for block in blocks[1..].iter() {
                insert_block(&mut state, block.clone()).unwrap();
            }

            // Get UTXO set without any pagination...
            let utxo_set = get_utxos(&state, &address.to_string(), 0, None, None)
                .unwrap()
                .utxos;

            // also get UTXO set with pagination until there are no
            // more pages returned...
            let mut utxos_chunked = vec![];
            let mut page = None;
            loop {
                let response = get_utxos(
                    &state,
                    &address.to_string(),
                    0,
                    page,
                    Some(utxo_limit),
                )
                .unwrap();
                utxos_chunked.extend(response.utxos);
                if response.next_page.is_none() {
                    break;
                } else {
                    page = response.next_page.map(|x| x.to_vec());
                }
            }

            // and compare the two results.
            assert_eq!(utxo_set, utxos_chunked);
        }
    }
}
