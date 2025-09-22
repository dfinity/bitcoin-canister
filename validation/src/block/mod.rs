#[cfg(test)]
mod tests;

use crate::{HeaderStore, HeaderValidator, ValidateHeaderError};
use bitcoin::{Network, Transaction};
use std::collections::BTreeSet;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum ValidateBlockError {
    NoTransactions,
    InvalidCoinbase,
    InvalidMerkleRoot,
    InvalidBlockHeader(ValidateHeaderError),
    DuplicateTransactions,
}

impl From<ValidateHeaderError> for ValidateBlockError {
    fn from(error: ValidateHeaderError) -> Self {
        Self::InvalidBlockHeader(error)
    }
}

pub struct BlockValidator<T> {
    header_validator: HeaderValidator<T>,
}

impl<T> BlockValidator<T> {
    pub fn new(store: T, network: Network) -> Self {
        BlockValidator {
            header_validator: HeaderValidator::new(store, network),
        }
    }
}

impl<T: HeaderStore> BlockValidator<T> {
    pub fn validate_block(
        &self,
        block: &bitcoin::Block,
        current_time: Duration,
    ) -> Result<(), ValidateBlockError> {
        validate_block(block).and_then(|()| {
            self.header_validator
                .validate_header(&block.header, current_time)
                .map_err(ValidateBlockError::InvalidBlockHeader)
        })
    }
}

fn validate_block(block: &bitcoin::Block) -> Result<(), ValidateBlockError> {
    // Check block like in
    // [bitcoin](https://github.com/rust-bitcoin/rust-bitcoin/blob/674ac57bce47e343d8f7c82e451aed5568766ba0/bitcoin/src/blockdata/block.rs#L126)
    let transactions = &block.txdata;
    if transactions.is_empty() {
        return Err(ValidateBlockError::NoTransactions);
    }

    if !transactions[0].is_coinbase() {
        return Err(ValidateBlockError::InvalidCoinbase);
    }

    if !block.check_merkle_root() {
        return Err(ValidateBlockError::InvalidMerkleRoot);
    }

    // TODO XC-497: evaluate performance impact of checking the witness commitment
    // like in [here](https://github.com/rust-bitcoin/rust-bitcoin/blob/674ac57bce47e343d8f7c82e451aed5568766ba0/bitcoin/src/blockdata/block.rs#L141)

    // Depart from the Rust bitcoin implementation because it's currently subject to
    // [CVE-2012-2459](https://bitcointalk.org/index.php?topic=102395)
    validate_unique_transactions(&block.txdata)?;

    Ok(())
}

fn validate_unique_transactions(transactions: &[Transaction]) -> Result<(), ValidateBlockError> {
    let mut unique_normalized_txids: BTreeSet<_> = BTreeSet::new();
    for tx in transactions {
        let unique = unique_normalized_txids.insert(tx.compute_ntxid());
        if !unique {
            return Err(ValidateBlockError::DuplicateTransactions);
        }
    }
    Ok(())
}
