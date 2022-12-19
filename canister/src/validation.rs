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
        let chain = unstable_blocks::get_chain_with_tip(
            &state.unstable_blocks,
            &header.prev_blockhash.into(),
        )
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
            self.state.stable_block_headers.get_with_height(height)
        } else if height <= self.height() {
            Some(*self.chain[(height - self.state.utxos.next_height()) as usize].header())
        } else {
            None
        }
    }
}
