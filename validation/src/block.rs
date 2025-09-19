use crate::{HeaderStore, HeaderValidator, ValidateHeaderError};
use bitcoin::Network;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum ValidateBlockError {
    NoTransactions,
    InvalidCoinbase,
    InvalidMerkleRoot,
    InvalidBlockHeader(ValidateHeaderError),
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
        //check block like in
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

        // TODO: witness commitment

        self.header_validator
            .validate_header(&block.header, current_time)
            .map_err(ValidateBlockError::InvalidBlockHeader)
    }
}
