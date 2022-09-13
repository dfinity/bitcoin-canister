use crate::{
    blocktree::{BlockChain, BlockDoesNotExtendTree},
    state::State,
    types::{OutPoint, Page, Slicing},
    unstable_blocks, utxoset,
};
use bitcoin::{Address, Block};
use ic_btc_types::{GetBalanceError, GetUtxosError, GetUtxosResponse, Height, Satoshi};
use serde_bytes::ByteBuf;
use std::str::FromStr;

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
    let chain_height = state.utxos.next_height + (chain.len() as u32) - 1;

    let mut tip_block_hash = chain.first().block_hash();
    let mut tip_block_height = state.utxos.next_height;

    // Apply unstable blocks to the UTXO set.
    for (i, block) in chain.into_chain().iter().enumerate() {
        let block_height = state.utxos.next_height + (i as u32);
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
/// multiple calls may be required for all the stable blocks to be ingested.
///
/// Returns a bool indicating whether or not the state has changed.
pub fn ingest_stable_blocks_into_utxoset(state: &mut State) -> bool {
    let prev_state = (
        state.utxos.next_height,
        &state.utxos.partial_stable_block.clone(),
    );
    let has_state_changed = |state: &State| -> bool {
        prev_state != (state.utxos.next_height, &state.utxos.partial_stable_block)
    };

    // Finish ingesting the stable block that's partially ingested, if that exists.
    match utxoset::ingest_block_continue(&mut state.utxos) {
        Slicing::Paused(()) => return has_state_changed(state),
        Slicing::Done => {}
    }

    // Check if there are any stable blocks and ingest those into the UTXO set.
    while let Some(new_stable_block) = unstable_blocks::pop(&mut state.unstable_blocks) {
        match utxoset::ingest_block(&mut state.utxos, new_stable_block) {
            Slicing::Paused(()) => return has_state_changed(state),
            Slicing::Done => {}
        }
    }

    has_state_changed(state)
}

pub fn main_chain_height(state: &State) -> Height {
    unstable_blocks::get_main_chain(&state.unstable_blocks).len() as u32 + state.utxos.next_height
        - 1
}

pub fn get_unstable_blocks(state: &State) -> Vec<&Block> {
    unstable_blocks::get_blocks(&state.unstable_blocks)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::random_p2pkh_address;
    use crate::types::Network;
    use bitcoin::secp256k1::rand::rngs::OsRng;
    use bitcoin::secp256k1::Secp256k1;
    use bitcoin::{Address, Network as BitcoinNetwork, PublicKey};
    use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};
    use ic_btc_types::{OutPoint, Utxo};
    use proptest::prelude::*;

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
