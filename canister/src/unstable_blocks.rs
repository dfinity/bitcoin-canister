mod outpoints_cache;

use crate::{
    blocktree::{BlockChain, BlockDoesNotExtendTree, BlockTree, Depth, DifficultyBasedDepth},
    runtime::print,
    types::{Address, TxOut},
    UtxoSet,
};
use bitcoin::block::Header;
use ic_btc_interface::{Height, Network};
use ic_btc_types::{Block, BlockHash, OutPoint};
use outpoints_cache::OutPointsCache;
use serde::{Deserialize, Serialize};

mod next_block_headers;
use self::next_block_headers::NextBlockHeaders;

/// Max allowed depth difference between the two longest branches
/// in the unstable block tree on `Testnet` and `Regtest`.
///
/// In these networks, difficulty resets to 1 if no block is found for
/// 20 minutes, leading to excessive chain growth before an anchor block
/// is considered stable.  
///
/// Without this limit, the unstable block tree could grow uncontrollably,
/// causing high memory usage and potentially leading to out-of-memory (OOM)
/// or stack overflow errors.  
///
/// This applies only to test environments and does not affect `Mainnet`.
pub const TESTNET_UNSTABLE_MAX_DEPTH_DIFFERENCE: Depth = Depth::new(1_000);

/// A data structure for maintaining all unstable blocks.
///
/// A block `b` is considered stable if:
///   depth(block) ≥ stability_threshold
///   ∀ b', height(b') = height(b): depth(b) - depth(b’) ≥ stability_threshold
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnstableBlocks {
    stability_threshold: u32,
    tree: BlockTree,
    outpoints_cache: OutPointsCache,
    network: Network,
    // The headers of the blocks that are expected to be received.
    next_block_headers: NextBlockHeaders,
}

impl UnstableBlocks {
    pub fn new(utxos: &UtxoSet, stability_threshold: u32, anchor: Block, network: Network) -> Self {
        // Create a cache of the transaction outputs, starting with the given anchor block.
        let mut outpoints_cache = OutPointsCache::new();
        outpoints_cache
            .insert(utxos, &anchor, utxos.next_height())
            .expect("anchor block must be valid.");

        Self {
            stability_threshold,
            tree: BlockTree::new(anchor.clone()),
            outpoints_cache,
            network,
            next_block_headers: NextBlockHeaders::default(),
        }
    }

    /// Retrieves the `TxOut` associated with the given `outpoint`, along with its height.
    pub fn get_tx_out(&self, outpoint: &OutPoint) -> Option<(&TxOut, Height)> {
        self.outpoints_cache.get_tx_out(outpoint)
    }

    /// Retrieves the list of outpoints that were added for the given address in the given block.
    pub fn get_added_outpoints(&self, block_hash: &BlockHash, address: &Address) -> &[OutPoint] {
        self.outpoints_cache
            .get_added_outpoints(block_hash, address)
    }

    /// Retrieves the list of outpoints that were removed for the given address in the given block.
    pub fn get_removed_outpoints(&self, block_hash: &BlockHash, address: &Address) -> &[OutPoint] {
        self.outpoints_cache
            .get_removed_outpoints(block_hash, address)
    }

    pub fn stability_threshold(&self) -> u32 {
        self.stability_threshold
    }

    pub fn set_stability_threshold(&mut self, stability_threshold: u32) {
        self.stability_threshold = stability_threshold;
    }

    pub fn anchor_difficulty(&self) -> u128 {
        self.tree.root.difficulty(self.network)
    }

    pub fn normalized_stability_threshold(&self) -> u128 {
        self.anchor_difficulty() * self.stability_threshold as u128
    }

    /// Returns the number of tips in the tree.
    pub fn tip_count(&self) -> u32 {
        self.tree.tip_count()
    }

    /// Returns the depths of all tips in the tree.
    pub fn tip_depths(&self) -> Vec<usize> {
        self.tree.tip_depths()
    }

    fn get_network(&self) -> Network {
        self.network
    }

    /// Returns all blocks in the tree with their respective depths
    /// separated by heights.
    pub fn blocks_with_depths_by_heights(&self) -> Vec<Vec<(&Block, u32)>> {
        self.tree.blocks_with_depths_by_heights()
    }

    /// Returns the depth of the unstable block tree.
    pub fn blocks_depth(&self) -> Depth {
        self.tree.depth()
    }

    /// Returns the difficulty-based depth of the unstable block tree.
    pub fn blocks_difficulty_based_depth(&self) -> DifficultyBasedDepth {
        self.tree.difficulty_based_depth(self.network)
    }

    /// Returns depth in BlockTree of Block with given BlockHash.
    fn block_depth(&mut self, block_hash: &BlockHash) -> Result<u32, BlockDoesNotExtendTree> {
        let (_, depth) = self
            .tree
            .find_mut(block_hash)
            .ok_or_else(|| BlockDoesNotExtendTree(block_hash.clone()))?;
        Ok(depth)
    }

    // Inserts the block header of the block that should be received.
    pub fn insert_next_block_header(
        &mut self,
        block_header: Header,
        stable_height: Height,
    ) -> Result<(), BlockDoesNotExtendTree> {
        let prev_block_hash = BlockHash::from(block_header.prev_blockhash);
        let height = match self.next_block_headers.get_height(&prev_block_hash) {
            Some(prev_height) => *prev_height,
            None => {
                if let Ok(depth) = self.block_depth(&prev_block_hash) {
                    stable_height + depth
                } else {
                    return Err(BlockDoesNotExtendTree(BlockHash::from(
                        block_header.block_hash(),
                    )));
                }
            }
        } + 1;
        self.next_block_headers.insert(block_header, height);
        Ok(())
    }

    /// Returns true if the given block header is already stored as one of the next block headers.
    pub fn has_next_block_header(&self, block_header: &Header) -> bool {
        self.next_block_headers
            .get_header(&BlockHash::from(block_header.block_hash()))
            .is_some()
    }

    // Public only for testing purpose.
    pub(crate) fn next_block_headers_max_height(&self) -> Option<Height> {
        self.next_block_headers.get_max_height()
    }

    // Returns Header chain from the tip up to the first block
    // header outside the main chain in the reverse order.
    pub fn get_next_block_headers_chain_with_tip(
        &self,
        tip_block_hash: BlockHash,
    ) -> Vec<(&Header, BlockHash)> {
        let mut chain = vec![];
        let mut curr_hash = tip_block_hash;
        while let Some(curr_header) = self.next_block_headers.get_header(&curr_hash) {
            chain.push((curr_header, curr_hash));
            curr_hash = BlockHash::from(curr_header.prev_blockhash);
        }
        chain.reverse();
        chain
    }

    /// Returns block headers of all unstable blocks in height range `heights`.
    pub fn get_block_headers_in_range(
        &self,
        stable_height: Height,
        heights: std::ops::RangeInclusive<Height>,
    ) -> impl Iterator<Item = &Header> {
        if *heights.end() < stable_height {
            // `stable_height` is larger than any height from the range, which implies none of the requested
            // blocks are in unstable blocks, hence the result should be an empty iterator.
            return Default::default();
        }

        // The last stable block is located in `unstable_blocks`, hence the height of the
        // first block in `unstable_blocks` is equal to `stable_height`.
        let heights_relative_to_unstable_blocks = std::ops::RangeInclusive::new(
            heights.start().saturating_sub(stable_height) as usize,
            heights.end().checked_sub(stable_height).unwrap() as usize,
        );

        get_main_chain(self).into_chain()[heights_relative_to_unstable_blocks]
            .iter()
            .map(|block| block.header())
            .collect::<Vec<_>>()
            .into_iter()
    }
}

/// Returns a reference to the `anchor` block iff ∃ a child `C` of `anchor` that is stable.
pub fn peek(blocks: &UnstableBlocks) -> Option<&Block> {
    get_stable_child(blocks).map(|_| &blocks.tree.root)
}

/// Pops the `anchor` block iff ∃ a child `C` of the `anchor` block that
/// is stable. The child `C` becomes the new `anchor` block, and all its
/// siblings are discarded.
pub fn pop(blocks: &mut UnstableBlocks, stable_height: Height) -> Option<Block> {
    let stable_child_idx = get_stable_child(blocks)?;

    let old_anchor = blocks.tree.root.clone();

    // Remove the outpoints of obsolete blocks from the cache.
    let obsolete_blocks: Vec<_> = blocks
        .tree
        .children
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != stable_child_idx)
        .flat_map(|(_, obsolete_child)| obsolete_child.blocks())
        .chain(std::iter::once(old_anchor.clone()))
        .collect();
    for block in obsolete_blocks {
        blocks.outpoints_cache.remove(&block);
    }

    // Replace the unstable block tree with that of the stable child.
    blocks.tree = blocks.tree.children.swap_remove(stable_child_idx);

    blocks.next_block_headers.remove_until_height(stable_height);

    Some(old_anchor)
}

/// Pushes a new block into the store.
pub fn push(
    blocks: &mut UnstableBlocks,
    utxos: &UtxoSet,
    block: Block,
) -> Result<(), BlockDoesNotExtendTree> {
    let (parent_block_tree, depth) = blocks
        .tree
        .find_mut(&block.header().prev_blockhash.into())
        .ok_or_else(|| BlockDoesNotExtendTree(block.block_hash()))?;

    let height = utxos.next_height() + depth + 1;

    blocks
        .outpoints_cache
        .insert(utxos, &block, height)
        .expect("inserting to outpoints cache must succeed.");

    let block_hash = block.block_hash();

    parent_block_tree.extend(block)?;

    blocks.next_block_headers.remove(&block_hash);

    Ok(())
}

/// Returns the best guess on what the main blockchain is.
///
/// The most likely chain to be "main", we hypothesize, is the longest
/// chain of blocks with an "uncontested" tip. As in, there exists no other
/// block at the same height as the tip.
pub fn get_main_chain(blocks: &UnstableBlocks) -> BlockChain {
    // Get all the blockchains that extend the anchor.
    let blockchains: Vec<BlockChain> = blocks.tree.blockchains();

    // Find the length of the longest blockchain.
    let mut longest_blockchain_len = 0;
    for blockchain in blockchains.iter() {
        longest_blockchain_len = longest_blockchain_len.max(blockchain.len());
    }

    // Get all the longest blockchains.
    let longest_blockchains: Vec<Vec<&'_ Block>> = blockchains
        .into_iter()
        .filter(|bc| bc.len() == longest_blockchain_len)
        .map(|bc| bc.into_chain())
        .collect();

    // A `BlockChain` contains at least one block which means we can safely index at
    // height 0 of the chain.
    let mut main_chain = BlockChain::new(longest_blockchains[0][0]);
    for height_idx in 1..longest_blockchain_len {
        // If all the blocks on the same height are identical, then this block is part of the
        // "main" chain.
        let block = longest_blockchains[0][height_idx];
        let block_hash = block.block_hash();
        for chain in longest_blockchains.iter().skip(1) {
            if chain[height_idx].block_hash() != block_hash {
                return main_chain;
            }
        }

        main_chain.push(block);
    }

    main_chain
}

/// Returns the length of the "main chain".
/// See `get_main_chain` for what defines a main chain.
pub fn get_main_chain_length(blocks: &UnstableBlocks) -> usize {
    let blocks_by_height = blocks.blocks_with_depths_by_heights();

    // Traverse the heights in reverse order. The highest height with a single block corresponds to
    // the tip of the main chain.
    for height in (0..blocks_by_height.len()).rev() {
        if blocks_by_height[height].len() == 1 {
            return height + 1;
        }
    }

    unreachable!("There must be at least one height with exactly one block.");
}

pub fn get_block_hashes(blocks: &UnstableBlocks) -> Vec<BlockHash> {
    blocks.tree.get_hashes()
}

pub fn blocks_count(blocks: &UnstableBlocks) -> usize {
    blocks.tree.blocks_count()
}

/// Returns a blockchain starting from the anchor and ending with the `tip`.
///
/// If the `tip` doesn't exist in the tree, `None` is returned.
pub fn get_chain_with_tip<'a>(
    blocks: &'a UnstableBlocks,
    tip: &BlockHash,
) -> Option<BlockChain<'a>> {
    blocks.tree.get_chain_with_tip(tip)
}

// Returns the index of the `anchor`'s stable child if it exists.
fn get_stable_child(blocks: &UnstableBlocks) -> Option<usize> {
    let network = blocks.get_network();

    // Compute and sort children by difficulty-based depth.
    let mut difficulty_based_depths: Vec<_> = blocks
        .tree
        .children
        .iter()
        .enumerate()
        .map(|(idx, child)| (child.difficulty_based_depth(network), idx))
        .collect();
    difficulty_based_depths.sort_by_key(|(depth, _)| *depth);

    let difficulty_based_stability_threshold =
        DifficultyBasedDepth::new(blocks.normalized_stability_threshold());

    let (difficulty_based_deepest_depth, child_idx) = difficulty_based_depths.last()?;

    // Prevent excessive chain growth in testnets where difficulty resets.
    if network == Network::Testnet || network == Network::Regtest {
        // The difficulty in the Bitcoin testnet/regtest can be reset to the minimum
        // in case a block hasn't been found for 20 minutes. This can be problematic.
        // Consider the following scenario:
        //
        // * Assume a `stability_threshold` of 144.
        // * The anchor at height `h` has difficulty of 4642.
        // * The anchor will be marked as stable if the difficulty-based depth of
        //   the successor blocks is `stability_threshold * difficulty(anchor)` = 668,448
        // * The difficulty is reset to the minimum of 1.
        // * The canister will now need to maintain a chain of length 668,448 just to
        //   mark the anchor block as stable!
        //
        // Very long chains can cause the stack to overflow, resulting in a broken
        // canister.
        //
        // The pragmatic solution in this case is to bound the length of the chain. If
        // one chain starts exceeding other chains by a certain length, we assume that
        // the anchor is stable even if the difficulty requirement hasn't been met.
        //
        // This scenario is only relevant for testnets, so this addition is safe and
        // has no impact on the behavior of the mainnet canister.
        let deepest_depth = blocks.tree.children[*child_idx].depth();
        if deepest_depth >= TESTNET_UNSTABLE_MAX_DEPTH_DIFFERENCE {
            // Ensure the second-longest chain is far enough behind.
            let second_deepest_depth = difficulty_based_depths
                .len()
                .checked_sub(2)
                .map(|idx| {
                    let (_, second_child_idx) = difficulty_based_depths[idx];
                    blocks.tree.children[second_child_idx].depth()
                })
                .unwrap_or(Depth::new(0));

            // NOTE: We use `saturating_sub` because `depths` is sorted by
            // `difficulty_based_depth`, but here we compare chains by `depth`.
            // This means `deepest_depth` may be smaller than `second_deepest_depth`,
            // so subtraction must not underflow.
            if deepest_depth.saturating_sub(second_deepest_depth)
                >= TESTNET_UNSTABLE_MAX_DEPTH_DIFFERENCE
            {
                print(&format!(
                    "Detected a chain that's > {TESTNET_UNSTABLE_MAX_DEPTH_DIFFERENCE} blocks ahead. \
                    Assuming its root is stable..."
                ));
                return Some(*child_idx);
            }
        }
    }

    // Ensure the deepest child meets the stability threshold.
    if *difficulty_based_deepest_depth < difficulty_based_stability_threshold {
        return None;
    }

    // If there are multiple children, ensure the longest chain is significantly ahead.
    if difficulty_based_depths.len() >= 2 {
        if let Some((difficulty_based_second_deepest_depth, _)) =
            difficulty_based_depths.get(difficulty_based_depths.len() - 2)
        {
            if *difficulty_based_deepest_depth - *difficulty_based_second_deepest_depth
                < difficulty_based_stability_threshold
            {
                // Difference must be >= difficulty_based_stability_threshold.
                return None;
            }
        }
    }

    Some(*child_idx)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{BlockBuilder, BlockChainBuilder};
    use ic_btc_interface::Network;
    use proptest::proptest;

    #[test]
    fn empty() {
        let anchor = BlockBuilder::genesis().build();
        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut unstable_blocks = UnstableBlocks::new(&utxos, 1, anchor, network);
        assert_eq!(peek(&unstable_blocks), None);
        assert_eq!(pop(&mut unstable_blocks, 0), None);
    }

    #[test]
    fn single_chain_same_difficulties() {
        let block_0 = BlockBuilder::genesis().build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();
        let network = Network::Regtest;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 2, block_0.clone(), network);

        push(&mut forest, &utxos, block_1).unwrap();
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);

        push(&mut forest, &utxos, block_2).unwrap();

        // Block 0 (the anchor) now has one stable child (Block 1).
        // Block 0 should be returned when calling `pop`.
        assert_eq!(peek(&forest), Some(&block_0));
        assert_eq!(pop(&mut forest, 0), Some(block_0));

        // Block 1 is now the anchor. It doesn't have stable
        // children yet, so calling `pop` should return `None`.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);
    }

    #[test]
    fn single_chain_various_difficulties() {
        let block_0 = BlockBuilder::genesis().build_with_mock_difficulty(20);
        let block_1 =
            BlockBuilder::with_prev_header(block_0.header()).build_with_mock_difficulty(15);
        let block_2 =
            BlockBuilder::with_prev_header(block_1.header()).build_with_mock_difficulty(20);
        let block_3 =
            BlockBuilder::with_prev_header(block_2.header()).build_with_mock_difficulty(110);

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 7, block_0.clone(), network);

        push(&mut forest, &utxos, block_1.clone()).unwrap();
        push(&mut forest, &utxos, block_2).unwrap();
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);

        push(&mut forest, &utxos, block_3).unwrap();
        // block_0 (the anchor) now has stable child block_1. Because block_1's
        // difficulty_based_depth is 15 + 20 + 110 = 145 is greater than
        // normalized_stability_threshold is 20 * 7 = 140 and it does not have
        // any siblings. Hence, block_0 should be returned when calling `pop`.
        assert_eq!(
            forest.tree.children[0].difficulty_based_depth(network),
            DifficultyBasedDepth::new(145)
        );

        assert_eq!(peek(&forest), Some(&block_0));
        assert_eq!(pop(&mut forest, 0), Some(block_0));

        // block_1 (the anchor) now has one stable child (block_2).
        // block_1 should be returned when calling `pop`.
        assert_eq!(peek(&forest), Some(&block_1));
        assert_eq!(pop(&mut forest, 0), Some(block_1));

        // block_2 is now the anchor. It doesn't have stable
        // children yet, so calling `pop` should return `None`.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);
    }

    #[test]
    fn forks_same_difficulties() {
        let genesis_block = BlockBuilder::genesis().build();
        let block = BlockBuilder::with_prev_header(genesis_block.header()).build();
        let forked_block = BlockBuilder::with_prev_header(genesis_block.header()).build();

        let network = Network::Regtest;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 2, genesis_block.clone(), network);

        push(&mut forest, &utxos, block).unwrap();
        push(&mut forest, &utxos, forked_block.clone()).unwrap();

        // None of the forks are stable, so we shouldn't get anything.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);

        // Extend fork2 by another block.
        let block_1 = BlockBuilder::with_prev_header(forked_block.header()).build();
        push(&mut forest, &utxos, block_1.clone()).unwrap();

        //Now, fork2 has a difficulty_based_depth of 2, while fork1 has a difficulty_based_depth of 1,
        //hence we cannot get a stable child.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);

        // Extend fork2 by another block.
        let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();
        push(&mut forest, &utxos, block_2).unwrap();
        //Now, fork2 has a difficulty_based_depth of 3, while fork1 has a difficulty_based_depth of 1,
        //hence we can get a stable child.
        assert_eq!(peek(&forest), Some(&genesis_block));
        assert_eq!(pop(&mut forest, 0), Some(genesis_block));
        assert_eq!(forest.tree.root, forked_block);

        //fork2 is still stable, hence we can get a stable child.
        assert_eq!(peek(&forest), Some(&forked_block));
        assert_eq!(pop(&mut forest, 0), Some(forked_block));
        assert_eq!(forest.tree.root, block_1);

        // No stable children for fork2.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);
    }

    #[test]
    fn forks_various_difficulties() {
        let genesis_block = BlockBuilder::genesis().build_with_mock_difficulty(4);
        let fork1_block =
            BlockBuilder::with_prev_header(genesis_block.header()).build_with_mock_difficulty(10);
        let fork2_block =
            BlockBuilder::with_prev_header(genesis_block.header()).build_with_mock_difficulty(5);

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 3, genesis_block.clone(), network);

        push(&mut forest, &utxos, fork1_block.clone()).unwrap();
        push(&mut forest, &utxos, fork2_block.clone()).unwrap();

        // None of the forks are stable, because fork1 has difficulty_based_depth of 10,
        // while fork2 has difficulty_based_depth 5, while normalized_stability_threshold
        // is 3 * 4 = 12. Hence, we shouldn't get anything.
        assert_eq!(
            forest.tree.children[0].difficulty_based_depth(network),
            DifficultyBasedDepth::new(10)
        );
        assert_eq!(
            forest.tree.children[1].difficulty_based_depth(network),
            DifficultyBasedDepth::new(5)
        );

        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);

        // Extend fork1 by another block.
        let block_1 =
            BlockBuilder::with_prev_header(fork1_block.header()).build_with_mock_difficulty(1);
        push(&mut forest, &utxos, block_1).unwrap();

        // Extend fork2 by another block.
        let block_2 =
            BlockBuilder::with_prev_header(fork2_block.header()).build_with_mock_difficulty(25);
        push(&mut forest, &utxos, block_2.clone()).unwrap();

        // Now, fork2 is stable becase its difficulty_based_depth is
        // 5 + 25 = 30 > normalized_stability_threshold, and fork1,
        // the only sibling of fork2, has difficulty_based_depth
        // 10 + 1 = 11, satisfying sencond condition
        // 30 - 11 > normalized_stability_threshold. So we can get a
        // stable child, and fork2_block should be a new anchor.
        assert_eq!(
            forest.tree.children[0].difficulty_based_depth(network),
            DifficultyBasedDepth::new(11)
        );
        assert_eq!(
            forest.tree.children[1].difficulty_based_depth(network),
            DifficultyBasedDepth::new(30)
        );

        assert_eq!(peek(&forest), Some(&genesis_block));
        assert_eq!(pop(&mut forest, 0), Some(genesis_block));
        assert_eq!(forest.tree.root, fork2_block);

        // fork2_block should have a stable child block_2, because
        // its difficulty_based_depth is 25,
        // normalized_stability_threshold is 3 * 5 = 15,
        // and it does not have any siblings.
        assert_eq!(
            forest.tree.children[0].difficulty_based_depth(network),
            DifficultyBasedDepth::new(25)
        );

        assert_eq!(peek(&forest), Some(&fork2_block));
        assert_eq!(pop(&mut forest, 0), Some(fork2_block));

        // No stable child for block_2, because it does not have any children.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);

        // Extend fork2 by another block.
        let block_3 =
            BlockBuilder::with_prev_header(block_2.header()).build_with_mock_difficulty(75);
        push(&mut forest, &utxos, block_3.clone()).unwrap();

        // Now block_2 has a stable child block_3, because its
        // difficulty_based_depth is 75, and
        // normalized_stability_threshold is 3 * 25 = 75,
        // hence difficulty_based_depth >= normalized_stability_threshold.
        assert_eq!(
            forest.tree.children[0].difficulty_based_depth(network),
            DifficultyBasedDepth::new(75)
        );

        assert_eq!(peek(&forest), Some(&block_2));
        assert_eq!(pop(&mut forest, 0), Some(block_2));
        assert_eq!(forest.tree.root, block_3);

        // No stable child for block_3, because it does not have any children.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);
    }

    #[test]
    fn insert_in_order() {
        let block_0 = BlockBuilder::genesis().build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 0, block_0.clone(), network);
        push(&mut forest, &utxos, block_1.clone()).unwrap();
        push(&mut forest, &utxos, block_2).unwrap();

        assert_eq!(peek(&forest), Some(&block_0));
        assert_eq!(pop(&mut forest, 0), Some(block_0));
        assert_eq!(peek(&forest), Some(&block_1));
        assert_eq!(pop(&mut forest, 0), Some(block_1));
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest, 0), None);
    }

    // Creating a forest that looks like this:
    //
    // * -> 1 -> 2
    //
    // Both blocks 1 and 2 are part of the main chain.
    #[test]
    fn get_main_chain_single_blockchain() {
        let block_0 = BlockBuilder::genesis().build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 1, block_0.clone(), network);

        push(&mut forest, &utxos, block_1.clone()).unwrap();
        push(&mut forest, &utxos, block_2.clone()).unwrap();
        assert_eq!(
            get_main_chain(&forest),
            BlockChain::new_with_successors(&block_0, vec![&block_1, &block_2])
        );
    }

    // Creating a forest that looks like this:
    //
    // * -> 1
    // * -> 2
    //
    // Both blocks 1 and 2 contest with each other -> main chain is empty.
    #[test]
    fn get_main_chain_two_contesting_trees() {
        let block_0 = BlockBuilder::genesis().build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_0.header()).build();

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 1, block_0.clone(), network);

        push(&mut forest, &utxos, block_1).unwrap();
        push(&mut forest, &utxos, block_2).unwrap();
        assert_eq!(get_main_chain(&forest), BlockChain::new(&block_0));
    }

    // Creating the following forest:
    //
    // * -> 1
    // * -> 2 -> 3
    //
    // "2 -> 3" is the longest blockchain and is should be considered "main".
    #[test]
    fn get_main_chain_longer_fork() {
        let block_0 = BlockBuilder::genesis().build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_3 = BlockBuilder::with_prev_header(block_2.header()).build();

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 1, block_0.clone(), network);

        push(&mut forest, &utxos, block_1).unwrap();
        push(&mut forest, &utxos, block_2.clone()).unwrap();
        push(&mut forest, &utxos, block_3.clone()).unwrap();
        assert_eq!(
            get_main_chain(&forest),
            BlockChain::new_with_successors(&block_0, vec![&block_2, &block_3])
        );
    }

    // Creating the following forest:
    //
    // * -> 1 -> 2 -> 3
    //       \-> a -> b
    //
    // "1" should be returned in this case, as its the longest chain
    // without a contested tip.
    #[test]
    fn get_main_chain_fork_at_first_block() {
        let block_0 = BlockBuilder::genesis().build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();
        let block_3 = BlockBuilder::with_prev_header(block_2.header()).build();
        let block_a = BlockBuilder::with_prev_header(block_1.header()).build();
        let block_b = BlockBuilder::with_prev_header(block_a.header()).build();

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 1, block_0.clone(), network);

        push(&mut forest, &utxos, block_1.clone()).unwrap();
        push(&mut forest, &utxos, block_2).unwrap();
        push(&mut forest, &utxos, block_3).unwrap();
        push(&mut forest, &utxos, block_a).unwrap();
        push(&mut forest, &utxos, block_b).unwrap();
        assert_eq!(
            get_main_chain(&forest),
            BlockChain::new_with_successors(&block_0, vec![&block_1])
        );
    }

    // Creating the following forest:
    //
    // * -> 1 -> 2 -> 3
    //       \-> a -> b
    //   -> x -> y -> z
    //
    // All blocks are contested.
    //
    // Then add block `c` that extends block `b`, at that point
    // `1 -> a -> b -> c` becomes the only longest chain, and therefore
    // the "main" chain.
    #[test]
    fn get_main_chain_multiple_forks() {
        let block_0 = BlockBuilder::genesis().build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();
        let block_3 = BlockBuilder::with_prev_header(block_2.header()).build();
        let block_a = BlockBuilder::with_prev_header(block_1.header()).build();
        let block_b = BlockBuilder::with_prev_header(block_a.header()).build();
        let block_x = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_y = BlockBuilder::with_prev_header(block_x.header()).build();
        let block_z = BlockBuilder::with_prev_header(block_y.header()).build();

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 1, block_0.clone(), network);

        push(&mut forest, &utxos, block_x).unwrap();
        push(&mut forest, &utxos, block_y).unwrap();
        push(&mut forest, &utxos, block_z).unwrap();
        push(&mut forest, &utxos, block_1.clone()).unwrap();
        push(&mut forest, &utxos, block_2).unwrap();
        push(&mut forest, &utxos, block_3).unwrap();
        push(&mut forest, &utxos, block_a.clone()).unwrap();
        push(&mut forest, &utxos, block_b.clone()).unwrap();
        assert_eq!(get_main_chain(&forest), BlockChain::new(&block_0));

        // Now add block c to b.
        let block_c = BlockBuilder::with_prev_header(block_b.header()).build();
        push(&mut forest, &utxos, block_c.clone()).unwrap();

        // Now the main chain should be "1 -> a -> b -> c"
        assert_eq!(
            get_main_chain(&forest),
            BlockChain::new_with_successors(&block_0, vec![&block_1, &block_a, &block_b, &block_c])
        );
    }

    // Same as the above test, with a different insertion order.
    #[test]
    fn get_main_chain_multiple_forks_2() {
        let block_0 = BlockBuilder::genesis().build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();
        let block_3 = BlockBuilder::with_prev_header(block_2.header()).build();
        let block_a = BlockBuilder::with_prev_header(block_1.header()).build();
        let block_b = BlockBuilder::with_prev_header(block_a.header()).build();
        let block_x = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_y = BlockBuilder::with_prev_header(block_x.header()).build();
        let block_z = BlockBuilder::with_prev_header(block_y.header()).build();

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 1, block_0.clone(), network);

        push(&mut forest, &utxos, block_1).unwrap();
        push(&mut forest, &utxos, block_2).unwrap();
        push(&mut forest, &utxos, block_3).unwrap();
        push(&mut forest, &utxos, block_a).unwrap();
        push(&mut forest, &utxos, block_b).unwrap();
        push(&mut forest, &utxos, block_x).unwrap();
        push(&mut forest, &utxos, block_y).unwrap();
        push(&mut forest, &utxos, block_z).unwrap();
        assert_eq!(get_main_chain(&forest), BlockChain::new(&block_0));
    }

    #[test]
    fn get_main_chain_anchor_only() {
        let block_0 = BlockBuilder::genesis().build();
        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let forest = UnstableBlocks::new(&utxos, 1, block_0.clone(), network);

        assert_eq!(get_main_chain(&forest), BlockChain::new(&block_0));
    }

    #[test]
    fn test_get_next_block_headers_chain_with_tip() {
        let genesis = BlockBuilder::genesis().build();
        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut unstable_blocks = UnstableBlocks::new(&utxos, 2, genesis.clone(), network);

        let block_0 = BlockBuilder::with_prev_header(genesis.header()).build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();
        let block_3 = BlockBuilder::with_prev_header(block_2.header()).build();
        let block_x = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_y = BlockBuilder::with_prev_header(block_x.header()).build();
        let block_z = BlockBuilder::with_prev_header(block_y.header()).build();

        unstable_blocks
            .insert_next_block_header(*block_0.header(), 0)
            .unwrap();
        unstable_blocks
            .insert_next_block_header(*block_1.header(), 0)
            .unwrap();
        unstable_blocks
            .insert_next_block_header(*block_2.header(), 0)
            .unwrap();
        unstable_blocks
            .insert_next_block_header(*block_3.header(), 0)
            .unwrap();
        unstable_blocks
            .insert_next_block_header(*block_x.header(), 0)
            .unwrap();
        unstable_blocks
            .insert_next_block_header(*block_y.header(), 0)
            .unwrap();
        unstable_blocks
            .insert_next_block_header(*block_z.header(), 0)
            .unwrap();

        assert_eq!(
            unstable_blocks.get_next_block_headers_chain_with_tip(
                BlockBuilder::with_prev_header(block_y.header())
                    .build()
                    .block_hash()
            ),
            vec![]
        );
        assert_eq!(
            unstable_blocks.get_next_block_headers_chain_with_tip(block_3.block_hash()),
            vec![
                (block_0.header(), block_0.block_hash()),
                (block_1.header(), block_1.block_hash()),
                (block_2.header(), block_2.block_hash()),
                (block_3.header(), block_3.block_hash())
            ]
        );
        assert_eq!(
            unstable_blocks.get_next_block_headers_chain_with_tip(block_y.block_hash()),
            vec![
                (block_0.header(), block_0.block_hash()),
                (block_x.header(), block_x.block_hash()),
                (block_y.header(), block_y.block_hash()),
            ]
        );
    }

    #[test]
    fn anchor_of_testnet_chain_longer_than_max_depth_is_marked_stable() {
        let stability_threshold = 144;
        let chain_len = 2000;
        let anchor_block_difficulty = 4642;
        let remaining_blocks_difficulty = 1;
        let network = Network::Regtest;
        let utxos = UtxoSet::new(network);

        // Assert the chain that will be built exceeds the maximum allowed, so that we can test
        // that case.
        assert!(Depth::new(chain_len) > TESTNET_UNSTABLE_MAX_DEPTH_DIFFERENCE);

        // Build a long chain where the first block has a substantially higher difficulty than the
        // remaining blocks.
        let chain = BlockChainBuilder::new(chain_len as u32)
            // Set the difficulty of the anchor block to be high.
            .with_difficulty(anchor_block_difficulty, 0..1)
            // Set the difficulty of the remaining blocks to be low.
            .with_difficulty(remaining_blocks_difficulty, 1..)
            .build();

        let mut unstable_blocks =
            UnstableBlocks::new(&utxos, stability_threshold, chain[0].clone(), network);

        // Sanity check that the difficulties are set correctly.
        assert_eq!(chain[0].mock_difficulty, Some(anchor_block_difficulty));
        assert_eq!(chain[1].mock_difficulty, Some(remaining_blocks_difficulty));
        assert_eq!(
            chain[chain_len as usize - 1].mock_difficulty,
            Some(remaining_blocks_difficulty)
        );

        // Insert chain into the state.
        for block in chain.iter().skip(1) {
            push(&mut unstable_blocks, &utxos, block.clone()).unwrap();
        }

        // The normalized stability threshold is now very high because of the high difficulty
        // of the anchor block.
        assert_eq!(
            unstable_blocks.normalized_stability_threshold(),
            anchor_block_difficulty * stability_threshold as u128
        );

        // The normalized stability threshold is still not met, which means that, in theory,
        // there are no stable blocks that can be popped.
        assert!(
            unstable_blocks.blocks_difficulty_based_depth()
                < DifficultyBasedDepth::new(unstable_blocks.normalized_stability_threshold())
        );

        assert_eq!(unstable_blocks.blocks_depth(), Depth::new(chain_len));

        // Even though the chain's difficulty-based depth doesn't exceed the normalized stability
        // threshold, the anchor block can now be popped because the chain's length has exceeded
        // the maximum allowed.
        assert_eq!(peek(&unstable_blocks), Some(&chain[0]));
    }

    #[test]
    fn long_testnet_chain_along_with_a_fork() {
        let stability_threshold = 144;
        let chain_len = 2000;
        let anchor_block_difficulty = 4642;
        let remaining_blocks_difficulty = 1;
        let network = Network::Regtest;
        let utxos = UtxoSet::new(network);

        // Assert the chain that will be built exceeds the maximum allowed, so that we can test
        // that case.
        assert!(Depth::new(chain_len) > TESTNET_UNSTABLE_MAX_DEPTH_DIFFERENCE);

        // Build a long chain where the first block has a substantially higher difficulty than the
        // remaining blocks.
        let chain = BlockChainBuilder::new(chain_len as u32)
            // Set the difficulty of the anchor block to be high.
            .with_difficulty(anchor_block_difficulty, 0..1)
            // Set the difficulty of the remaining blocks to be low.
            .with_difficulty(remaining_blocks_difficulty, 1..)
            .build();

        // Build a second chain that's a fork of the first.
        let second_chain = BlockChainBuilder::fork(
            &chain[0],
            TESTNET_UNSTABLE_MAX_DEPTH_DIFFERENCE.get() as u32 - 1,
        )
        .with_difficulty(remaining_blocks_difficulty, 0..)
        .build();

        let mut unstable_blocks =
            UnstableBlocks::new(&utxos, stability_threshold, chain[0].clone(), network);

        // Insert chains into the state.
        for block in chain.iter().skip(1) {
            push(&mut unstable_blocks, &utxos, block.clone()).unwrap();
        }
        for block in second_chain.iter() {
            push(&mut unstable_blocks, &utxos, block.clone()).unwrap();
        }

        // The normalized stability threshold is still not met, which means that, in theory,
        // there are no stable blocks that can be popped.
        assert!(
            unstable_blocks.blocks_difficulty_based_depth()
                < DifficultyBasedDepth::new(unstable_blocks.normalized_stability_threshold())
        );

        // If there's a very long testnet chain `A`, and there exists another chain `B` s.t.
        // depth(A) - depth(B) < TESTNET_UNSTABLE_MAX_DEPTH_DIFFERENCE,
        // the root of chain `A` is considered stable.
        assert_eq!(peek(&unstable_blocks), Some(&chain[0]));

        // Add one more block to the second chain, so that it's depth
        // is `TESTNET_UNSTABLE_MAX_DEPTH_DIFFERENCE`.
        push(
            &mut unstable_blocks,
            &utxos,
            BlockBuilder::with_prev_header(second_chain.last().unwrap().header()).build(),
        )
        .unwrap();

        // Now, depth(A) - depth(B) >= TESTNET_UNSTABLE_MAX_DEPTH_DIFFERENCE
        // and the root of chain `A` is considered unstable.
        assert_eq!(peek(&unstable_blocks), None);
    }

    fn get_block_headers_helper(block_num: usize) -> (UnstableBlocks, Vec<Header>) {
        let mut headers = vec![];
        let block_0 = BlockBuilder::genesis().build();
        headers.push(*block_0.header());

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut unstable_blocks = UnstableBlocks::new(&utxos, 1, block_0.clone(), network);

        for i in 1..block_num {
            let block = BlockBuilder::with_prev_header(&headers[i - 1]).build();
            headers.push(*block.header());
            push(&mut unstable_blocks, &utxos, block).unwrap();
        }

        (unstable_blocks, headers)
    }

    #[test]
    fn test_get_block_headers_in_range_in_stable_blocks() {
        let block_num = 15;

        let (unstable_blocks, _) = get_block_headers_helper(block_num);

        let stable_height = 10;
        let range = std::ops::RangeInclusive::new(0, stable_height - 1);

        // `stable_height` is larger than any height from the range, which implies none of the requested
        // blocks are in unstable blocks, hence the result should be an empty iterator.
        assert!(unstable_blocks
            .get_block_headers_in_range(stable_height, range)
            .eq([].iter()));
    }

    #[test]
    fn test_get_block_headers_in_range() {
        let block_num = 100;

        let (unstable_blocks, headers) = get_block_headers_helper(block_num);

        proptest!(|(
            start_range in 0..=block_num - 1,
            range_length in 1..=block_num)|{
                let end_range = std::cmp::min(start_range + range_length - 1, block_num - 1 );

                let mut result = unstable_blocks.get_block_headers_in_range(0, std::ops::RangeInclusive::new(start_range as u32, end_range as u32)).peekable();

                for expected_result in headers.iter().take(end_range + 1).skip(start_range){
                    assert_eq!(expected_result, *result.peek().unwrap());
                    result.next();
                }
            }
        );
    }
}
