use ic_btc_interface::Network;
use ic_btc_types::{Block, BlockHash};
use std::fmt;
mod serde;

/// Represents a non-empty block chain as:
/// * the first block of the chain
/// * the successors to this block (which can be an empty list)
#[derive(Debug, PartialEq, Eq)]
pub struct BlockChain<'a> {
    // The first block of this `BlockChain`, i.e. the one at the lowest height.
    first: &'a Block,
    // The successor blocks of this `BlockChain`, i.e. the chain after the
    // `first` block.
    successors: Vec<&'a Block>,
}

impl<'a> BlockChain<'a> {
    /// Creates a new `BlockChain` with the given `first` block and an empty list
    /// of successors.
    pub fn new(first: &'a Block) -> Self {
        Self {
            first,
            successors: vec![],
        }
    }

    /// This is only useful for tests to simplify the creation of a `BlockChain`.
    #[cfg(test)]
    pub fn new_with_successors(first: &'a Block, successors: Vec<&'a Block>) -> Self {
        Self { first, successors }
    }

    /// Appends a new block to the list of `successors` of this `BlockChain`.
    pub fn push(&mut self, block: &'a Block) {
        self.successors.push(block);
    }

    /// Returns the length of this `BlockChain`.
    pub fn len(&self) -> usize {
        self.successors.len() + 1
    }

    pub fn first(&self) -> &'a Block {
        self.first
    }

    pub fn tip(&self) -> &'a Block {
        match self.successors.last() {
            None => {
                // The chain consists of only one block, and that is the tip.
                self.first
            }
            Some(tip) => tip,
        }
    }

    /// Consumes this `BlockChain` and returns the entire chain of blocks.
    pub fn into_chain(self) -> Vec<&'a Block> {
        let mut chain = vec![self.first];
        chain.extend(self.successors);
        chain
    }
}

/// Error returned when attempting to create a `BlockChain` out of an empty
/// list of blocks.
#[derive(Debug)]
pub struct EmptyChainError {}

impl fmt::Display for EmptyChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot create a `BlockChain` from an empty chain")
    }
}

/// Maintains a tree of connected blocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockTree {
    pub root: Block,
    pub children: Vec<BlockTree>,
}

impl BlockTree {
    /// Creates a new `BlockTree` with the given block as its root.
    pub fn new(root: Block) -> Self {
        Self {
            root,
            children: vec![],
        }
    }

    /// Returns all blocks in the tree with their depths
    /// separated by heights.
    pub fn blocks_with_depths_by_heights(&self) -> Vec<Vec<(&Block, u32)>> {
        let mut blocks_with_depths_by_heights: Vec<Vec<(&Block, u32)>> = vec![vec![]];
        self.blocks_with_depths_by_heights_helper(&mut blocks_with_depths_by_heights, 0);
        blocks_with_depths_by_heights
    }

    fn blocks_with_depths_by_heights_helper<'a>(
        &'a self,
        blocks_with_depth_by_height: &mut Vec<Vec<(&'a Block, u32)>>,
        height: usize,
    ) -> u32 {
        let mut depth: u32 = 0;
        for child in self.children.iter() {
            depth = std::cmp::max(
                depth,
                child.blocks_with_depths_by_heights_helper(blocks_with_depth_by_height, height + 1),
            );
        }
        depth += 1;

        if height >= blocks_with_depth_by_height.len() {
            blocks_with_depth_by_height.resize(height + 1, vec![]);
        }

        blocks_with_depth_by_height[height].push((&self.root, depth));

        depth
    }

    /// Returns the number of tips in the tree.
    pub fn num_tips(&self) -> u32 {
        if self.children.is_empty() {
            1
        } else {
            self.children.iter().map(|c| c.num_tips()).sum()
        }
    }
}

/// Extends the tree with the given block.
///
/// Blocks can extend the tree in the following cases:
///   * The block is already present in the tree (no-op).
///   * The block is a successor of a block already in the tree.
pub fn extend(block_tree: &mut BlockTree, block: Block) -> Result<(), BlockDoesNotExtendTree> {
    if contains(block_tree, &block) {
        // The block is already present in the tree. Nothing to do.
        return Ok(());
    }

    // Check if the block is a successor to any of the blocks in the tree.
    match find_mut(block_tree, &block.header().prev_blockhash.into()) {
        Some((block_subtree, _)) => {
            assert_eq!(
                block_subtree.root.block_hash().to_vec(),
                block.header().prev_blockhash.to_vec()
            );
            // Add the block as a successor.
            block_subtree.children.push(BlockTree::new(block));
            Ok(())
        }
        None => Err(BlockDoesNotExtendTree(block.block_hash())),
    }
}

/// Returns all the blockchains in the tree.
pub fn blockchains(block_tree: &BlockTree) -> Vec<BlockChain> {
    if block_tree.children.is_empty() {
        return vec![BlockChain {
            first: &block_tree.root,
            successors: vec![],
        }];
    }

    let mut tips = vec![];
    for child in block_tree.children.iter() {
        tips.extend(
            blockchains(child)
                .into_iter()
                .map(|bc| BlockChain {
                    first: &block_tree.root,
                    successors: bc.into_chain(),
                })
                .collect::<Vec<BlockChain>>(),
        );
    }

    tips
}

/// Returns a `BlockChain` starting from the anchor and ending with the `tip`.
///
/// If the `tip` doesn't exist in the tree, `None` is returned.
pub fn get_chain_with_tip<'a>(
    block_tree: &'a BlockTree,
    tip: &BlockHash,
) -> Option<BlockChain<'a>> {
    // Compute the chain in reverse order, as that's more efficient, and then
    // reverse it to get the answer in the correct order.
    get_chain_with_tip_reverse(block_tree, tip).map(|mut chain| {
        // Safe to unwrap as the `chain` would contain at least the root of the
        // `BlockTree` it was produced from.
        // This would be the first block since the chain is in reverse order.
        let first = chain.pop().unwrap();
        // Reverse the chain to get the list of `successors` in the right order.
        chain.reverse();
        BlockChain {
            first,
            successors: chain,
        }
    })
}

// Do a depth-first search to find the blockchain that ends with the given `tip`.
// For performance reasons, the list is returned in the reverse order, starting
// from `tip` and ending with `anchor`.
fn get_chain_with_tip_reverse<'a>(
    block_tree: &'a BlockTree,
    tip: &BlockHash,
) -> Option<Vec<&'a Block>> {
    if block_tree.root.block_hash() == *tip {
        return Some(vec![&block_tree.root]);
    }

    for child in block_tree.children.iter() {
        if let Some(mut chain) = get_chain_with_tip_reverse(child, tip) {
            chain.push(&block_tree.root);
            return Some(chain);
        }
    }

    None
}

// Returns the maximum sum of block difficulties from the root to a leaf inclusive.
pub fn difficulty_based_depth(tree: &BlockTree, network: Network) -> u128 {
    let mut res: u128 = 0;
    for child in tree.children.iter() {
        res = std::cmp::max(res, difficulty_based_depth(child, network));
    }
    res += tree.root.difficulty(network) as u128;
    res
}

pub fn depth(tree: &BlockTree) -> u128 {
    let mut res: u128 = 0;
    for child in tree.children.iter() {
        res = std::cmp::max(res, depth(child));
    }
    res += 1;
    res
}

// Returns a `BlockTree` where the hash of the root block matches the provided `block_hash`
// along with its depth if it exists, and `None` otherwise.
pub fn find_mut<'a>(
    block_tree: &'a mut BlockTree,
    blockhash: &BlockHash,
) -> Option<(&'a mut BlockTree, u32)> {
    fn find_mut_helper<'a>(
        block_tree: &'a mut BlockTree,
        blockhash: &BlockHash,
        depth: u32,
    ) -> Option<(&'a mut BlockTree, u32)> {
        if block_tree.root.block_hash() == *blockhash {
            return Some((block_tree, depth));
        }

        for child in block_tree.children.iter_mut() {
            if let res @ Some(_) = find_mut_helper(child, blockhash, depth + 1) {
                return res;
            }
        }

        None
    }

    find_mut_helper(block_tree, blockhash, 0)
}

// Returns true if a block exists in the tree, false otherwise.
fn contains(block_tree: &BlockTree, block: &Block) -> bool {
    if block_tree.root.block_hash() == block.block_hash() {
        return true;
    }

    for child in block_tree.children.iter() {
        if contains(child, block) {
            return true;
        }
    }

    false
}

/// An error thrown when trying to add a block that isn't a successor
/// of any block in the tree.
#[derive(Debug)]
pub struct BlockDoesNotExtendTree(pub BlockHash);

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{BlockBuilder, BlockChainBuilder};

    #[test]
    fn tree_single_block() {
        let block_tree = BlockTree::new(BlockBuilder::genesis().build());

        assert_eq!(
            blockchains(&block_tree),
            vec![BlockChain {
                first: &block_tree.root,
                successors: vec![],
            }]
        );
    }

    #[test]
    fn tree_multiple_forks() {
        let genesis_block = BlockBuilder::genesis().build();
        let genesis_block_header = *genesis_block.header();
        let mut block_tree = BlockTree::new(genesis_block);

        for i in 1..5 {
            // Create different blocks extending the genesis block.
            // Each one of these should be a separate fork.
            extend(
                &mut block_tree,
                BlockBuilder::with_prev_header(&genesis_block_header).build(),
            )
            .unwrap();
            assert_eq!(blockchains(&block_tree).len(), i);
        }

        assert_eq!(block_tree.children.len(), 4);
    }

    #[test]
    fn chain_with_tip_no_forks() {
        let mut blocks = vec![BlockBuilder::genesis().build()];
        for i in 1..10 {
            blocks.push(BlockBuilder::with_prev_header(blocks[i - 1].header()).build())
        }

        let mut block_tree = BlockTree::new(blocks[0].clone());

        for block in blocks.iter() {
            extend(&mut block_tree, block.clone()).unwrap();
        }

        for (i, block) in blocks.iter().enumerate() {
            // Fetch the blockchain with the `block` as tip.
            let block_hash = block.block_hash();
            let chain = get_chain_with_tip(&block_tree, &block_hash)
                .unwrap()
                .into_chain();

            // The first block should be the genesis block.
            assert_eq!(chain[0], &blocks[0]);
            // The last block should be the expected tip.
            assert_eq!(chain.last().unwrap(), &block);

            // The length of the chain should grow as the requested tip gets deeper.
            assert_eq!(chain.len(), i + 1);

            // All blocks should be correctly chained to one another.
            for i in 1..chain.len() {
                assert_eq!(
                    chain[i - 1].block_hash().to_vec(),
                    chain[i].header().prev_blockhash.to_vec()
                )
            }
        }
    }

    #[test]
    fn chain_with_tip_multiple_forks() {
        let mut blocks = vec![BlockBuilder::genesis().build()];
        let mut block_tree = BlockTree::new(blocks[0].clone());

        let num_forks = 5;
        for _ in 0..num_forks {
            for i in 1..10 {
                blocks.push(BlockBuilder::with_prev_header(blocks[i - 1].header()).build())
            }

            for block in blocks.iter() {
                extend(&mut block_tree, block.clone()).unwrap();
            }

            for (i, block) in blocks.iter().enumerate() {
                // Fetch the blockchain with the `block` as tip.
                let block_hash = block.block_hash();
                let chain = get_chain_with_tip(&block_tree, &block_hash)
                    .unwrap()
                    .into_chain();

                // The first block should be the genesis block.
                assert_eq!(chain[0], &blocks[0]);
                // The last block should be the expected tip.
                assert_eq!(chain.last().unwrap(), &block);

                // The length of the chain should grow as the requested tip gets deeper.
                assert_eq!(chain.len(), i + 1);

                // All blocks should be correctly chained to one another.
                for i in 1..chain.len() {
                    assert_eq!(
                        chain[i - 1].block_hash().to_vec(),
                        chain[i].header().prev_blockhash.to_vec()
                    )
                }
            }

            blocks = vec![blocks[0].clone()];
        }
    }

    #[test]
    fn test_difficulty_based_depth_single_block() {
        let block_tree = BlockTree::new(BlockBuilder::genesis().build_with_mock_difficulty(5));

        assert_eq!(difficulty_based_depth(&block_tree, Network::Mainnet), 5);
    }

    #[test]
    fn test_difficulty_based_depth_root_with_children() {
        let genesis_block = BlockBuilder::genesis().build_with_mock_difficulty(5);
        let genesis_block_header = *genesis_block.header();
        let mut block_tree = BlockTree::new(genesis_block);

        for i in 1..11 {
            extend(
                &mut block_tree,
                BlockBuilder::with_prev_header(&genesis_block_header).build_with_mock_difficulty(i),
            )
            .unwrap();
        }

        // The maximum sum of block difficulties from the root to a leaf is the sum
        // of the root and child with the greatest difficulty which is 5 + 10 = 15.
        assert_eq!(difficulty_based_depth(&block_tree, Network::Mainnet), 15);
    }

    #[test]
    fn test_blocks_with_depths_by_heights_only_root() {
        let genesis_block = BlockBuilder::genesis().build();
        let block_tree = BlockTree::new(genesis_block.clone());
        let blocks_with_depths_by_heights = block_tree.blocks_with_depths_by_heights();

        // The number of rows in blocks_with_depths_by_heights should be 1.
        // The row should have only 1 column.
        assert_eq!(blocks_with_depths_by_heights.len(), 1);
        assert_eq!(blocks_with_depths_by_heights[0].len(), 1);

        let (block, depth) = blocks_with_depths_by_heights[0][0];
        // Depth of the genesis block should be 1.
        assert_eq!(block.block_hash(), genesis_block.block_hash());
        assert_eq!(depth, 1);
    }

    #[test]
    fn test_blocks_with_depths_by_heights_chain() {
        let chain_len: usize = 10;
        let chain = BlockChainBuilder::new(chain_len as u32).build();

        let mut block_tree = BlockTree::new(chain[0].clone());

        let mut expected_blocks_with_depths_by_heights: Vec<Vec<(&Block, u32)>> =
            vec![vec![]; chain_len];

        for (i, block) in chain.iter().enumerate() {
            expected_blocks_with_depths_by_heights[i].push((block, (chain_len - i) as u32));
            extend(&mut block_tree, block.clone()).unwrap();
        }

        let actual_blocks_with_depths_by_heights = block_tree.blocks_with_depths_by_heights();

        assert_eq!(chain_len, actual_blocks_with_depths_by_heights.len());

        for i in 0..chain_len {
            // On each height, actual_blocks_with_depths_by_heights should have only 1 block.
            assert_eq!(actual_blocks_with_depths_by_heights[i].len(), 1);
            let (expected_block, expected_depth) = expected_blocks_with_depths_by_heights[i][0];
            let (acutal_block, actual_depth) = actual_blocks_with_depths_by_heights[i][0];
            assert_eq!(expected_block.block_hash(), acutal_block.block_hash());
            assert_eq!(expected_depth, actual_depth);
        }
    }

    #[test]
    fn test_blocks_with_depths_by_heights_fork() {
        let chain = BlockChainBuilder::new(2).build();
        // Create a fork from the genesis block with length 2.
        let fork = BlockChainBuilder::fork(&chain[0], 2).build();

        let mut block_tree = BlockTree::new(chain[0].clone());
        extend(&mut block_tree, chain[1].clone()).unwrap();
        extend(&mut block_tree, fork[0].clone()).unwrap();
        extend(&mut block_tree, fork[1].clone()).unwrap();

        let blocks_with_depths_by_heights = block_tree.blocks_with_depths_by_heights();

        // blocks_with_depths_by_heights should have 3 heights.
        assert_eq!(blocks_with_depths_by_heights.len(), 3);

        // On height 0, blocks_with_depths_by_heights should have only one block.
        assert_eq!(blocks_with_depths_by_heights[0].len(), 1);

        let (height_0_block, height_0_depth) = blocks_with_depths_by_heights[0][0];
        assert_eq!(height_0_block.block_hash(), chain[0].block_hash());
        assert_eq!(height_0_depth, 3);

        // On height 1, blocks_with_depths_by_heights should have two blocks.
        assert_eq!(blocks_with_depths_by_heights[1].len(), 2);

        let (first_block_height_1, _) = blocks_with_depths_by_heights[1][0];
        let (second_block_height_1, _) = blocks_with_depths_by_heights[1][1];

        // Check that blocks are different.
        assert_ne!(
            first_block_height_1.block_hash(),
            second_block_height_1.block_hash()
        );

        for (block, depth) in blocks_with_depths_by_heights[1].iter() {
            if block.block_hash() == chain[1].block_hash() {
                assert_eq!(*depth, 1);
            } else if block.block_hash() == fork[0].block_hash() {
                assert_eq!(*depth, 2);
            } else {
                panic!("Unexpected block.");
            }
        }

        // On height 2, blocks_with_depths_by_heights should have only one block.
        assert_eq!(blocks_with_depths_by_heights[2].len(), 1);

        let (height_2_block, height_2_depth) = blocks_with_depths_by_heights[2][0];

        assert_eq!(height_2_block.block_hash(), fork[1].block_hash());
        assert_eq!(height_2_depth, 1);
    }
}
