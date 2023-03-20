use crate::{blocktree::BlockDoesNotExtendTree, state::State, unstable_blocks, Block};
use bitcoin::BlockHeader;
use ic_btc_validation::HeaderStore;

/// A structure passed to the validation crate to validate a specific block header.
pub struct ValidationContext<'a> {
    state: &'a State,
    chain: Vec<&'a Block>,
}

impl<'a> ValidationContext<'a> {
    /// Initialize a `ValidationContext` for the given block header.
    pub fn new(state: &'a State, header: &BlockHeader) -> Result<Self, BlockDoesNotExtendTree> {
        // Retrieve the chain that the given header extends.
        // The given header must extend one of the unstable blocks.
        let prev_block_hash = header.prev_blockhash.into();
        let chain = unstable_blocks::get_chain_with_tip(&state.unstable_blocks, &prev_block_hash)
            .ok_or_else(|| BlockDoesNotExtendTree(header.block_hash().into()))?
            .into_chain();

        Ok(Self { state, chain })
    }
}

/// Implements the `HeaderStore` trait that's used for validating headers.
impl<'a> HeaderStore for ValidationContext<'a> {
    fn get_with_block_hash(&self, hash: &bitcoin::BlockHash) -> Option<BlockHeader> {
        // Check if the header is in the chain.
        let hash = crate::types::BlockHash::from(hash.to_vec());
        for block in self.chain.iter() {
            if block.block_hash() == hash {
                return Some(*block.header());
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

    fn get_with_height(&self, height: u32) -> Option<BlockHeader> {
        if height < self.state.utxos.next_height() {
            // The height requested is for a stable block.
            // Retrieve the block header from the stable block headers.
            self.state.stable_block_headers.get_with_height(height)
        } else if height <= self.height() {
            // The height requested is for an unstable block.
            // Retrieve the block header from the chain.
            Some(*self.chain[(height - self.state.utxos.next_height()) as usize].header())
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
        test_utils::build_chain,
        types::Network,
    };
    use proptest::prelude::*;
    use std::str::FromStr;

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
