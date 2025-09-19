use crate::{validate_header, HeaderStore, ValidateHeaderError};
use bitcoin::Network;

#[derive(Debug, PartialEq)]
pub enum ValidateBlockError {
    NoTransactions,
    InvalidCoinbase,
    InvalidMerkleRoot,
    InvalidBlockHeader(ValidateHeaderError),
}

pub fn validate_block(
    network: &Network,
    store: &impl HeaderStore,
    block: bitcoin::Block,
    current_time: u64,
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

    validate_header(network, store, &block.header, current_time)
        .map_err(ValidateBlockError::InvalidBlockHeader)
}
