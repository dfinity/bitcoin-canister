mod outpoints_cache;
use crate::{
    blocktree::{self, BlockChain, BlockDoesNotExtendTree, BlockTree},
    runtime::print,
    types::{Address, TxOut},
    UtxoSet,
};
use bitcoin::BlockHeader;
use ic_btc_interface::{Height, Network};
use ic_btc_types::{Block, BlockHash, OutPoint};
use outpoints_cache::OutPointsCache;
use serde::{Deserialize, Serialize};

mod next_block_headers;
use self::next_block_headers::NextBlockHeaders;

const TESTNET_MAX_SOLO_CHAIN_LENGTH: u128 = 1000;

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
    /// The headers of the blocks that are expected to be received.
    // TODO(EXC-1379): remove this directive once it's deployed to production.
    #[serde(default)]
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

    pub fn anchor_difficulty(&self) -> u64 {
        self.tree.root.difficulty(self.network)
    }

    pub fn normalized_stability_threshold(&self) -> u128 {
        self.anchor_difficulty() as u128 * self.stability_threshold as u128
    }

    /// Returns the number of tips available in the current block tree.
    pub fn num_tips(&self) -> u32 {
        self.tree.num_tips()
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
    pub fn blocks_depth(&self) -> u128 {
        blocktree::depth(&self.tree)
    }

    /// Returns the difficulty-based depth of the unstable block tree.
    pub fn blocks_difficulty_based_depth(&self) -> u128 {
        blocktree::difficulty_based_depth(&self.tree, self.network)
    }

    /// Returns depth in BlockTree of Block with given BlockHash.
    fn block_depth(&mut self, block_hash: &BlockHash) -> Result<u32, BlockDoesNotExtendTree> {
        let (_, depth) = blocktree::find_mut(&mut self.tree, block_hash)
            .ok_or_else(|| BlockDoesNotExtendTree(block_hash.clone()))?;
        Ok(depth)
    }

    // Inserts the block header of the block that should be received.
    pub fn insert_next_block_header(
        &mut self,
        block_header: BlockHeader,
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
    pub fn has_next_block_header(&self, block_header: &BlockHeader) -> bool {
        self.next_block_headers
            .get_header(&BlockHash::from(block_header.block_hash()))
            .is_some()
    }

    // Public only for testing purpose.
    pub(crate) fn next_block_headers_max_height(&self) -> Option<Height> {
        self.next_block_headers.get_max_height()
    }

    // Returns BlockHeader chain from the tip up to the first block
    // header outside the main chain in the reverse order.
    pub fn get_next_block_headers_chain_with_tip(
        &self,
        tip_block_hash: BlockHash,
    ) -> Vec<(&BlockHeader, BlockHash)> {
        let mut chain = vec![];
        let mut curr_hash = tip_block_hash;
        while let Some(curr_header) = self.next_block_headers.get_header(&curr_hash) {
            chain.push((curr_header, curr_hash));
            curr_hash = BlockHash::from(curr_header.prev_blockhash);
        }
        chain.reverse();
        chain
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
    match get_stable_child(blocks) {
        Some(stable_child_idx) => {
            let old_anchor = blocks.tree.root.clone();

            // Replace the unstable block tree with that of the stable child.
            blocks.tree = blocks.tree.children.swap_remove(stable_child_idx);

            // Remove the outpoints of the old anchor from the cache.
            blocks.outpoints_cache.remove(&old_anchor);

            blocks.next_block_headers.remove_until_height(stable_height);

            Some(old_anchor)
        }
        None => None,
    }
}

/// Pushes a new block into the store.
pub fn push(
    blocks: &mut UnstableBlocks,
    utxos: &UtxoSet,
    block: Block,
) -> Result<(), BlockDoesNotExtendTree> {
    let (parent_block_tree, depth) =
        blocktree::find_mut(&mut blocks.tree, &block.header().prev_blockhash.into())
            .ok_or_else(|| BlockDoesNotExtendTree(block.block_hash()))?;

    let height = utxos.next_height() + depth + 1;

    blocks
        .outpoints_cache
        .insert(utxos, &block, height)
        .expect("inserting to outpoints cache must succeed.");

    let block_hash = block.block_hash();

    blocktree::extend(parent_block_tree, block)?;

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
    let blockchains: Vec<BlockChain> = blocktree::blockchains(&blocks.tree);

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

pub fn get_blocks(blocks: &UnstableBlocks) -> Vec<&Block> {
    blocktree::blockchains(&blocks.tree)
        .into_iter()
        .flat_map(|bc| bc.into_chain())
        .collect()
}

/// Returns a blockchain starting from the anchor and ending with the `tip`.
///
/// If the `tip` doesn't exist in the tree, `None` is returned.
pub fn get_chain_with_tip<'a>(
    blocks: &'a UnstableBlocks,
    tip: &BlockHash,
) -> Option<BlockChain<'a>> {
    blocktree::get_chain_with_tip(&blocks.tree, tip)
}

// Returns the index of the `anchor`'s stable child if it exists.
fn get_stable_child(blocks: &UnstableBlocks) -> Option<usize> {
    // Compute the difficulty based depth of all the children.
    let network = blocks.get_network();

    let mut depths: Vec<_> = blocks
        .tree
        .children
        .iter()
        .enumerate()
        .map(|(idx, child)| (blocktree::difficulty_based_depth(child, network), idx))
        .collect();

    // Sort by depth.
    depths.sort_by_key(|(depth, _child_idx)| *depth);

    let normalized_stability_threshold = blocks.normalized_stability_threshold();

    match depths.last() {
        Some((deepest_depth, child_idx)) => {
            match network {
                Network::Testnet | Network::Regtest => {
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
                    // Very long chains can cause the shadow stacks to overflow, resulting in a
                    // broken canister.
                    //
                    // The pragmatic solution in this case is to bound the length of the chain. If
                    // there's only one chain and it starts exceeding a certain length, we assume
                    // that the anchor is stable even if the difficulty requirement hasn't been
                    // met.
                    //
                    // This scenario is only relevant for testnets, so this addition is safe and
                    // has not impact on the behavior of the mainnet canister.
                    if depths.len() == 1
                        && blocktree::depth(&blocks.tree.children[*child_idx])
                            > TESTNET_MAX_SOLO_CHAIN_LENGTH
                    {
                        print(
                            "Detected a solo chain > {TESTNET_MAX_SOLO_CHAIN_LENGTH}. Assuming the root is stable...",
                        );
                        return Some(*child_idx);
                    }
                }
                Network::Mainnet => {
                    // The difficulty on mainnet is much more stable and is bounded to change by a
                    // factor of 4, so there is no limit that needs to be imposed.
                }
            }

            // The deepest child tree must have a depth >= normalized_stability_threshold.
            if *deepest_depth < normalized_stability_threshold {
                // Need a depth of at least >= normalized_stability_threshold.
                return None;
            }

            // If there is more than one child, the difference in depth
            // between the deepest child and all the others must be >= normalized_stability_threshold.
            if depths.len() >= 2 {
                if let Some((second_deepest_depth, _)) = depths.get(depths.len() - 2) {
                    if deepest_depth - second_deepest_depth < normalized_stability_threshold {
                        // Difference must be >= normalized_stability_threshold.
                        return None;
                    }
                }
            }

            Some(*child_idx)
        }
        None => {
            // The anchor has no children. Nothing to return.
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{BlockBuilder, BlockChainBuilder};
    use ic_btc_interface::Network;

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
            crate::blocktree::difficulty_based_depth(&forest.tree.children[0], network),
            145
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
            crate::blocktree::difficulty_based_depth(&forest.tree.children[0], network),
            10
        );
        assert_eq!(
            crate::blocktree::difficulty_based_depth(&forest.tree.children[1], network),
            5
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
            crate::blocktree::difficulty_based_depth(&forest.tree.children[0], network),
            11
        );
        assert_eq!(
            crate::blocktree::difficulty_based_depth(&forest.tree.children[1], network),
            30
        );

        assert_eq!(peek(&forest), Some(&genesis_block));
        assert_eq!(pop(&mut forest, 0), Some(genesis_block));
        assert_eq!(forest.tree.root, fork2_block);

        // fork2_block should have a stable child block_2, because
        // its difficulty_based_depth is 25,
        // normalized_stability_threshold is 3 * 5 = 15,
        // and it does not have any siblings.
        assert_eq!(
            crate::blocktree::difficulty_based_depth(&forest.tree.children[0], network),
            25
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
            crate::blocktree::difficulty_based_depth(&forest.tree.children[0], network),
            75
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
    fn testnet_chain_longer_than_max_solo_chain() {
        let stability_threshold = 144;
        let chain_len = 2000;
        let anchor_block_difficulty = 4642;
        let remaining_blocks_difficulty = 1;
        let network = Network::Regtest;
        let utxos = UtxoSet::new(network);

        // Assert the chain that will be built exceeds the maximum allowed, so that we can test
        // that case.
        assert!(chain_len > TESTNET_MAX_SOLO_CHAIN_LENGTH);

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
            anchor_block_difficulty as u128 * stability_threshold as u128
        );

        // The normalized stability threshold is still not met, which means that, in theory,
        // there are no stable blocks that can be popped.
        assert!(
            unstable_blocks.blocks_difficulty_based_depth()
                < unstable_blocks.normalized_stability_threshold()
        );

        assert_eq!(unstable_blocks.blocks_depth(), chain_len);

        // Even though the chain's difficulty-based depth doesn't exceed the normalized stability
        // threshold, the anchor block can now be popped because the chain's length has exceeded
        // the maximum allowed.
        assert_eq!(peek(&unstable_blocks), Some(&chain[0]));
    }
}
