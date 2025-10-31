use crate::{state::State, unstable_blocks};
use bitcoin::{block::Header, hashes::Hash};
use ic_btc_types::BlockHash;
use ic_btc_validation::HeaderStore;

/// A structure passed to the validation crate to validate a specific block header.
pub struct ValidationContext<'a> {
    state: &'a State,
    // BlockHash is stored in order to avoid repeatedly calling to
    // Header::block_hash() which is expensive.
    chain: Vec<(&'a Header, ic_btc_types::BlockHash)>,
}

#[derive(Debug, PartialEq)]
pub enum ValidationContextError {
    BlockDoesNotExtendTree(BlockHash),
    AlreadyKnown(BlockHash),
}

impl<'a> ValidationContext<'a> {
    /// Initialize a `ValidationContext` for the given block header.
    pub fn new(state: &'a State, header: &Header) -> Result<Self, ValidationContextError> {
        // Retrieve the chain that the given header extends.
        // The given header must extend one of the unstable blocks.
        let prev_block_hash = header.prev_blockhash.into();
        let current_block_hash = ic_btc_types::BlockHash::from(header.block_hash());
        let (chain, tip_successors) =
            unstable_blocks::get_chain_with_tip(&state.unstable_blocks, &prev_block_hash)
                .ok_or_else(|| {
                    ValidationContextError::BlockDoesNotExtendTree(current_block_hash)
                })?;
        if tip_successors
            .iter()
            .any(|c| c.block_hash() == &current_block_hash)
        {
            return Err(ValidationContextError::AlreadyKnown(current_block_hash));
        }
        let chain = chain
            .into_chain()
            .iter()
            .map(|block| (block.header(), *block.block_hash()))
            .collect();

        Ok(Self { state, chain })
    }

    /// Initialize a `ValidationContext` for the given block header.
    /// The given block header can be in the 'NextBlockHeaders'.
    pub fn new_with_next_block_headers(
        state: &'a State,
        header: &Header,
    ) -> Result<Self, ValidationContextError> {
        let prev_block_hash = header.prev_blockhash.into();
        let next_block_headers_chain = state
            .unstable_blocks
            .get_next_block_headers_chain_with_tip(&prev_block_hash);
        if next_block_headers_chain.is_empty() {
            Self::new(state, header)
        } else {
            let mut context = Self::new(state, next_block_headers_chain[0].0)?;
            for item in next_block_headers_chain.iter() {
                context.chain.push(item.clone())
            }
            Ok(context)
        }
    }
}

/// Implements the `HeaderStore` trait that's used for validating headers.
impl HeaderStore for ValidationContext<'_> {
    fn get_with_block_hash(&self, hash: &bitcoin::BlockHash) -> Option<Header> {
        // Check if the header is in the chain.
        let hash = ic_btc_types::BlockHash::from(hash.as_raw_hash().as_byte_array().to_vec());
        for item in self.chain.iter() {
            if item.1 == hash {
                return Some(*item.0);
            }
        }

        // The header is in the stable store.
        self.state.stable_block_headers.get_with_block_hash(&hash)
    }

    fn height(&self) -> u32 {
        // The `next_height` method returns the height of the UTXOs + 1, so we
        // subtract 1 to account for that.
        self.state.utxos.next_height() + self.chain.len() as u32 - 1
    }

    fn get_with_height(&self, height: u32) -> Option<Header> {
        if height < self.state.utxos.next_height() {
            // The height requested is for a stable block.
            // Retrieve the block header from the stable block headers.
            self.state.stable_block_headers.get_with_height(height)
        } else if height <= self.height() {
            // The height requested is for an unstable block.
            // Retrieve the block header from the chain.
            Some(*self.chain[(height - self.state.utxos.next_height()) as usize].0)
        } else {
            // The height requested is higher than the tip.
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        state::{ingest_stable_blocks_into_utxoset, insert_block},
        test_utils::{build_chain, BlockBuilder},
    };
    use ic_btc_interface::Network;
    use proptest::prelude::*;
    use std::str::FromStr;

    #[test]
    fn test_new_with_next_block_headers() {
        let genesis = BlockBuilder::genesis().build();
        let network = Network::Mainnet;

        let mut state = State::new(2, network, genesis.clone());
        let block_0 = BlockBuilder::with_prev_header(genesis.header()).build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();
        state
            .unstable_blocks
            .insert_next_block_header(*block_0.header(), 0)
            .unwrap();
        state
            .unstable_blocks
            .insert_next_block_header(*block_1.header(), 0)
            .unwrap();
        state
            .unstable_blocks
            .insert_next_block_header(*block_2.header(), 0)
            .unwrap();

        let block_3 = BlockBuilder::with_prev_header(block_2.header()).build();

        let validation_context =
            ValidationContext::new_with_next_block_headers(&state, block_3.header()).unwrap();

        assert_eq!(
            validation_context.chain,
            vec![
                (genesis.header(), *genesis.block_hash()),
                (block_0.header(), *block_0.block_hash()),
                (block_1.header(), *block_1.block_hash()),
                (block_2.header(), *block_2.block_hash()),
            ]
        );

        let not_inserted_1 = BlockBuilder::with_prev_header(genesis.header()).build();
        let not_inserted_2 = BlockBuilder::with_prev_header(not_inserted_1.header()).build();

        assert!(matches!(
            ValidationContext::new_with_next_block_headers(&state, not_inserted_2.header()),
            Err(ValidationContextError::BlockDoesNotExtendTree(..))
        ));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]
        #[test]
        fn validation_context(
            stability_threshold in 1..150u32,
            num_blocks in 2..250u32,
        ) {
            let num_transactions_in_block = 1;
            let network = Network::Regtest;
            let blocks = build_chain(network, num_blocks, num_transactions_in_block);

            let mut state = State::new(stability_threshold, network, blocks[0].clone());

            // Insert all the blocks except the last block.
            for block in blocks[1..blocks.len() - 1].iter() {
                insert_block(&mut state, block.clone()).unwrap();
                ingest_stable_blocks_into_utxoset(&mut state);
            }

            // Try validating the last block header (which wasn't inserted above).
            let validation_context =
                ValidationContext::new(&state, blocks[blocks.len() - 1].header()).unwrap();

            // Assert the height is correct.
            assert_eq!(validation_context.height(), blocks.len() as u32 - 2);

            // Assert that getting a header with a given height is correct.
            for height in 0..num_blocks - 1 {
                assert_eq!(
                    validation_context.get_with_height(height),
                    Some(*blocks[height as usize].header())
                );
            }
            assert_eq!(validation_context.get_with_height(num_blocks - 1), None);

            // Assert that getting a header with a given block hash is correct.
            for height in 0..num_blocks - 1 {
                assert_eq!(
                    validation_context.get_with_block_hash(
                        &bitcoin::BlockHash::from_str(
                            &blocks[height as usize].block_hash().to_string()
                        )
                        .unwrap()
                    ),
                    Some(*blocks[height as usize].header())
                );
            }
            assert_eq!(
                validation_context.get_with_block_hash(
                    &bitcoin::BlockHash::from_str(
                        &blocks[(num_blocks - 1) as usize].block_hash().to_string()
                    )
                    .unwrap()
                ),
                None
            );
        }
    }
}
