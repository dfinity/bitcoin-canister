use crate::types::Network;
use bitcoin::{secp256k1::rand::rngs::OsRng, secp256k1::Secp256k1, Address, Block, PublicKey};
use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};
use stable_structures::{Memory, StableBTreeMap, Storable};

/// Generates a random P2PKH address.
pub fn random_p2pkh_address(network: Network) -> Address {
    let secp = Secp256k1::new();
    let mut rng = OsRng::new().unwrap();

    Address::p2pkh(
        &PublicKey::new(secp.generate_keypair(&mut rng).1),
        network.into(),
    )
}

/// Builds a random chain with the given number of block and transactions.
pub fn build_chain(
    network: Network,
    num_blocks: u32,
    num_transactions_per_block: u32,
) -> Vec<Block> {
    let address = random_p2pkh_address(network);
    let mut prev_block: Option<Block> = None;
    let mut blocks = vec![];
    let mut value = 1;

    for _ in 0..num_blocks {
        let mut block_builder = match prev_block {
            Some(b) => BlockBuilder::with_prev_header(b.header),
            None => BlockBuilder::genesis(),
        };

        let mut transactions = vec![];
        for _ in 0..num_transactions_per_block {
            transactions.push(
                TransactionBuilder::coinbase()
                    .with_output(&address, value)
                    .build(),
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
    blocks
}

/// Returns true if the instances of `StableBTreeMap` provided are equal.
pub fn is_stable_btreemap_equal<M: Memory + Clone, K: Storable + Eq, V: Storable + Eq>(
    a: &StableBTreeMap<M, K, V>,
    b: &StableBTreeMap<M, K, V>,
) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for (x, y) in a.iter().zip(b.iter()) {
        if x != y {
            return false;
        }
    }

    true
}
