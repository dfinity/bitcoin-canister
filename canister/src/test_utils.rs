use crate::types::Network;
use bitcoin::{
    blockdata::constants::genesis_block, secp256k1::rand::rngs::OsRng, secp256k1::Secp256k1,
    Address, Block, PublicKey,
};
use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};
use ic_stable_structures::{Memory, StableBTreeMap, Storable};

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
/// The genesis block used in the chain is also random.
pub fn build_chain(
    network: Network,
    num_blocks: u32,
    num_transactions_per_block: u32,
) -> Vec<Block> {
    build_chain_with_genesis_block(
        network,
        BlockBuilder::genesis().build(),
        num_blocks,
        num_transactions_per_block,
    )
}

/// Builds a random chain with the given number of block and transactions
/// and starting with the Regtest genesis block.
pub fn build_regtest_chain(num_blocks: u32, num_transactions_per_block: u32) -> Vec<Block> {
    let network = Network::Regtest;
    build_chain_with_genesis_block(
        network,
        genesis_block(network.into()),
        num_blocks,
        num_transactions_per_block,
    )
}

fn build_chain_with_genesis_block(
    network: Network,
    genesis_block: Block,
    num_blocks: u32,
    num_transactions_per_block: u32,
) -> Vec<Block> {
    let address = random_p2pkh_address(network);
    let mut blocks = vec![genesis_block.clone()];
    let mut prev_block: Block = genesis_block;
    let mut value = 1;

    // Since we start with a genesis block, we need `num_blocks - 1` additional blocks.
    for _ in 0..num_blocks - 1 {
        let mut block_builder = BlockBuilder::with_prev_header(prev_block.header);
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
        prev_block = block;
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
