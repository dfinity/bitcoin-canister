use crate::{
    genesis_block,
    types::{Block, Network},
};
use bitcoin::{
    secp256k1::rand::rngs::OsRng, secp256k1::Secp256k1, Address, BlockHeader, PublicKey,
    Transaction,
};
use ic_btc_test_utils::{BlockBuilder as ExternalBlockBuilder, TransactionBuilder};
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
        genesis_block(network),
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
        let mut block_builder = BlockBuilder::with_prev_header(prev_block.header());
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

/// A wrapper around `ic_btc_test_utils::BlockBuilder` that returns `crate::types::Block`
/// as opposed to `bitcoin::Block`.
pub struct BlockBuilder {
    builder: ExternalBlockBuilder,
}

impl BlockBuilder {
    pub fn genesis() -> Self {
        Self {
            builder: ExternalBlockBuilder::genesis(),
        }
    }

    pub fn with_prev_header(prev_header: &BlockHeader) -> Self {
        Self {
            builder: ExternalBlockBuilder::with_prev_header(*prev_header),
        }
    }

    pub fn with_transaction(self, transaction: Transaction) -> Self {
        Self {
            builder: self.builder.with_transaction(transaction),
        }
    }

    pub fn build(self) -> Block {
        Block::new(self.builder.build())
    }
}
