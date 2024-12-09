use crate::{
    genesis_block,
    types::{into_bitcoin_network, Address},
};
use bitcoin::{
    block::Header, Address as BitcoinAddress, CompressedPublicKey, PublicKey, Script, Witness,
};
use ic_btc_interface::Network;
use ic_btc_test_utils::{
    BlockBuilder as ExternalBlockBuilder, TransactionBuilder as ExternalTransactionBuilder,
};
use ic_btc_types::{Block, OutPoint, Transaction};
use ic_stable_structures::{Memory, StableBTreeMap, Storable};
use secp256k1::{
    rand::{rngs::OsRng, RngCore},
    Secp256k1,
};
use std::{
    convert::TryFrom,
    ops::{Bound, RangeBounds},
    str::FromStr,
};

/// Generates a random P2PKH address.
pub fn random_p2pkh_address(network: Network) -> Address {
    let secp = Secp256k1::new();

    BitcoinAddress::p2pkh(
        PublicKey::new(secp.generate_keypair(&mut OsRng).1),
        into_bitcoin_network(network),
    )
    .into()
}

pub fn random_p2tr_address(network: Network) -> Address {
    ic_btc_test_utils::random_p2tr_address(into_bitcoin_network(network)).into()
}

pub fn random_p2wpkh_address(network: Network) -> Address {
    let secp = Secp256k1::new();
    BitcoinAddress::p2wpkh(
        &CompressedPublicKey::try_from(PublicKey::new(secp.generate_keypair(&mut OsRng).1))
            .expect("failed to create p2wpkh address"),
        into_bitcoin_network(network),
    )
    .into()
}

pub fn random_p2wsh_address(network: Network) -> Address {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    BitcoinAddress::p2wsh(
        &Script::from_bytes(&bytes).to_p2wsh(),
        into_bitcoin_network(network),
    )
    .into()
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
pub fn is_stable_btreemap_equal<M: Memory, K: Storable + Ord + Eq + Clone, V: Storable + Eq>(
    a: &StableBTreeMap<K, V, M>,
    b: &StableBTreeMap<K, V, M>,
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
    mock_difficulty: Option<u128>,
}

impl BlockBuilder {
    pub fn genesis() -> Self {
        Self {
            builder: ExternalBlockBuilder::genesis(),
            mock_difficulty: None,
        }
    }

    pub fn with_prev_header(prev_header: &Header) -> Self {
        Self {
            builder: ExternalBlockBuilder::with_prev_header(*prev_header),
            mock_difficulty: None,
        }
    }

    pub fn with_transaction(self, transaction: Transaction) -> Self {
        Self {
            builder: self.builder.with_transaction(transaction.into()),
            mock_difficulty: None,
        }
    }

    pub fn with_difficulty(self, difficulty: u128) -> Self {
        Self {
            mock_difficulty: Some(difficulty),
            ..self
        }
    }

    pub fn build(self) -> Block {
        let mut block = Block::new(self.builder.build());
        block.mock_difficulty = self.mock_difficulty;
        block
    }

    pub fn build_with_mock_difficulty(self, mock_difficulty: u128) -> Block {
        let mut block = self.build();
        block.mock_difficulty = Some(mock_difficulty);
        block
    }
}

/// A wrapper around `ic_btc_test_utils::TransactionBuilder` that returns
/// `crate::types::Transaction` as opposed to `bitcoin::Transaction`.
pub struct TransactionBuilder {
    builder: ExternalTransactionBuilder,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            builder: ExternalTransactionBuilder::new(),
        }
    }

    pub fn coinbase() -> Self {
        Self {
            builder: ExternalTransactionBuilder::coinbase(),
        }
    }

    pub fn with_input(self, previous_output: OutPoint) -> Self {
        Self {
            builder: self.builder.with_input(previous_output.into(), None),
        }
    }

    pub fn with_input_and_witness(self, previous_output: OutPoint, witness: Witness) -> Self {
        Self {
            builder: self
                .builder
                .with_input(previous_output.into(), Some(witness)),
        }
    }

    pub fn with_lock_time(self, i: u32) -> Self {
        Self {
            builder: self.builder.with_lock_time(i),
        }
    }

    pub fn with_output(self, address: &Address, value: u64) -> Self {
        Self {
            builder: self.builder.with_output(
                &BitcoinAddress::from_str(&address.to_string())
                    .map(|a| a.assume_checked())
                    .unwrap(),
                value,
            ),
        }
    }

    pub fn build(self) -> Transaction {
        Transaction::new(self.builder.build())
    }
}

pub struct BlockChainBuilder {
    num_blocks: u32,
    prev_block_header: Option<Header>,
    #[allow(clippy::type_complexity)]
    difficulty_ranges: Vec<((Bound<usize>, Bound<usize>), u128)>,
}

impl BlockChainBuilder {
    pub fn new(num_blocks: u32) -> Self {
        Self {
            num_blocks,
            prev_block_header: None,
            difficulty_ranges: vec![],
        }
    }

    pub fn fork(prev_block: &Block, num_blocks: u32) -> Self {
        Self {
            num_blocks,
            prev_block_header: Some(*prev_block.header()),
            difficulty_ranges: vec![],
        }
    }

    /// Sets the difficulty of blocks at the given range of heights.
    pub fn with_difficulty<R: RangeBounds<usize>>(mut self, difficulty: u128, range: R) -> Self {
        self.difficulty_ranges.push((
            (range.start_bound().cloned(), range.end_bound().cloned()),
            difficulty,
        ));
        self
    }

    pub fn build(self) -> Vec<Block> {
        let mut blocks = Vec::with_capacity(self.num_blocks as usize);

        let mut first_block = match self.prev_block_header {
            None => genesis_block(Network::Regtest),
            Some(prev_block_header) => BlockBuilder::with_prev_header(&prev_block_header).build(),
        };
        if let difficulty @ Some(_) = self.get_difficulty(0) {
            first_block.mock_difficulty = difficulty;
        }

        blocks.push(first_block);

        for i in 1..self.num_blocks as usize {
            let mut block = BlockBuilder::with_prev_header(blocks[i - 1].header());
            if let Some(difficulty) = self.get_difficulty(i) {
                block = block.with_difficulty(difficulty);
            }
            blocks.push(block.build());
        }

        blocks
    }

    fn get_difficulty(&self, i: usize) -> Option<u128> {
        for (range, difficulty) in &self.difficulty_ranges {
            if range.contains(&i) {
                return Some(*difficulty);
            }
        }
        None
    }
}

#[test]
fn target_difficulty() {
    // Regtest blocks by the BlockBuilder should have a difficulty of 1.
    assert_eq!(
        Block::target_difficulty(
            Network::Regtest,
            BlockBuilder::genesis().build().header().target()
        ),
        1
    );
}
