use ic_btc_interface::Network;
use ic_btc_types::{Block, BlockHash};
use std::fmt;
mod serde;
use std::ops::{Add, Sub};

/// Represents a non-empty block chain as:
/// * the first block of the chain
/// * the successors to this block (which can be an empty list)
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Clone))]
pub struct BlockChain<'a, Block> {
    // The first block of this `BlockChain`, i.e. the one at the lowest height.
    first: &'a Block,
    // The successor blocks of this `BlockChain`, i.e. the chain after the
    // `first` block.
    successors: Vec<&'a Block>,
}

impl<'a, Block> BlockChain<'a, Block> {
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

/// Depth of a blockchain, measured in the number of blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Depth(u64);

impl Depth {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn get(self) -> u64 {
        self.0
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl Add for Depth {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl fmt::Display for Depth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Depth based on accumulated difficulty, used for block stability checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DifficultyBasedDepth(u128);

impl DifficultyBasedDepth {
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    pub fn get(self) -> u128 {
        self.0
    }
}

impl Add for DifficultyBasedDepth {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Sub for DifficultyBasedDepth {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

/// Maintains a tree of connected blocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockTree<Block> {
    root: Block,
    children: Vec<BlockTree<Block>>,
}

impl<Block> BlockTree<Block> {
    /// Creates a new `BlockTree` with the given block as its root.
    pub fn new(root: Block) -> Self {
        Self {
            root,
            children: vec![],
        }
    }

    pub fn root(&self) -> &Block {
        &self.root
    }

    pub fn children(&self) -> &[Self] {
        &self.children
    }

    pub fn remove_child(&mut self, idx: usize) -> Self {
        self.children.swap_remove(idx)
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
    pub fn tip_count(&self) -> u32 {
        if self.children.is_empty() {
            1
        } else {
            self.children.iter().map(|c| c.tip_count()).sum()
        }
    }

    /// Returns the depths of all tips in the tree.
    pub fn tip_depths(&self) -> Vec<usize> {
        if self.children.is_empty() {
            return vec![1]; // Leaf node, depth is 1
        }

        self.children
            .iter()
            .flat_map(|child| child.tip_depths().into_iter().map(|d| d + 1))
            .collect()
    }

    /// Returns all the blockchains in the tree.
    pub fn blockchains(&self) -> Vec<BlockChain<'_, Block>> {
        if self.children.is_empty() {
            return vec![BlockChain {
                first: &self.root,
                successors: vec![],
            }];
        }

        let mut tips = vec![];
        for child in self.children.iter() {
            tips.extend(
                child
                    .blockchains()
                    .into_iter()
                    .map(|bc| BlockChain {
                        first: &self.root,
                        successors: bc.into_chain(),
                    })
                    .collect::<Vec<_>>(),
            );
        }

        tips
    }

    fn get_child_blocks(&self) -> Vec<&Block> {
        self.children.iter().map(|c| &c.root).collect()
    }

    /// Returns the number of blocks in the tree.
    pub fn blocks_count(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(|child| child.blocks_count())
            .sum::<usize>()
    }

    /// Returns all blocks in the tree.
    pub fn blocks(&self) -> Vec<&Block> {
        let mut blocks = vec![&self.root];
        for child in self.children.iter() {
            blocks.extend(child.blocks());
        }
        blocks
    }
}

pub trait ChainBlock {
    fn block_hash(&self) -> &BlockHash;
    fn prev_block_hash(&self) -> BlockHash;
    fn difficulty(&self, network: Network) -> u128;
}

impl ChainBlock for Block {
    fn block_hash(&self) -> &BlockHash {
        self.block_hash()
    }
    fn prev_block_hash(&self) -> BlockHash {
        BlockHash::from(self.header().prev_blockhash)
    }
    fn difficulty(&self, network: Network) -> u128 {
        self.difficulty(network)
    }
}

impl<Block: ChainBlock> BlockTree<Block> {
    /// Extends the tree with the given block.
    ///
    /// Blocks can extend the tree in the following cases:
    ///   * The block is a successor of a block already in the tree.
    ///
    /// Note that `ValidationContext` ensures that the block to insert is not already present.
    pub fn extend(&mut self, block: Block) -> Result<(), BlockDoesNotExtendTree> {
        debug_assert!(
            self.find(block.block_hash()).is_none(),
            "BUG: block {} is already present in the tree, but this should have been prevented when instantiating `ValidationContext`",
            block.block_hash()
        );

        // Check if the block is a successor to any of the blocks in the tree.
        let prev_block_hash = block.prev_block_hash();
        match self.find_mut(&prev_block_hash) {
            Some((block_subtree, _)) => {
                assert_eq!(block_subtree.root.block_hash(), &prev_block_hash);
                // Add the block as a successor.
                block_subtree.children.push(BlockTree::new(block));
                Ok(())
            }
            None => Err(BlockDoesNotExtendTree(block.block_hash().clone())),
        }
    }

    /// Returns a `BlockChain` starting from the anchor and ending with the `tip`,
    /// together with the tip's direct children.
    ///
    /// If the `tip` doesn't exist in the tree, `None` is returned.
    pub fn get_chain_with_tip<'a>(
        &'a self,
        tip: &BlockHash,
    ) -> Option<(BlockChain<'a, Block>, Vec<&'a Block>)> {
        // Compute the chain in reverse order, as that's more efficient, and then
        // reverse it to get the answer in the correct order.
        self.get_chain_with_tip_reverse(tip)
            .map(|(mut chain, tip_successors)| {
                // Safe to unwrap as the `chain` would contain at least the root of the
                // `BlockTree` it was produced from.
                // This would be the first block since the chain is in reverse order.
                let first = chain.pop().unwrap();
                // Reverse the chain to get the list of `successors` in the right order.
                chain.reverse();
                (
                    BlockChain {
                        first,
                        successors: chain,
                    },
                    tip_successors,
                )
            })
    }

    // Do a depth-first search to find the blockchain that ends with the given `tip`.
    // For performance reasons, the list is returned in the reverse order, starting
    // from `tip` and ending with `anchor`.
    fn get_chain_with_tip_reverse<'a>(
        &'a self,
        tip: &BlockHash,
    ) -> Option<(Vec<&'a Block>, Vec<&'a Block>)> {
        if self.root.block_hash() == tip {
            return Some((vec![&self.root], self.get_child_blocks()));
        }

        for child in self.children.iter() {
            if let Some((mut chain, tip_successors)) = child.get_chain_with_tip_reverse(tip) {
                chain.push(&self.root);
                return Some((chain, tip_successors));
            }
        }

        None
    }

    // Returns the maximum sum of block difficulties from the root to a leaf inclusive.
    pub fn difficulty_based_depth(&self, network: Network) -> DifficultyBasedDepth {
        let mut res = DifficultyBasedDepth::new(0);
        for child in self.children.iter() {
            res = std::cmp::max(res, child.difficulty_based_depth(network));
        }
        res = res + DifficultyBasedDepth::new(self.root.difficulty(network));
        res
    }

    pub fn depth(&self) -> Depth {
        let mut res = Depth::new(0);
        for child in self.children.iter() {
            res = std::cmp::max(res, child.depth());
        }
        res = res + Depth::new(1);
        res
    }

    /// Returns a `BlockTree` where the hash of the root block matches the provided `block_hash`
    /// along with its depth if it exists, and `None` otherwise.
    pub fn find_mut<'a>(
        &'a mut self,
        blockhash: &BlockHash,
    ) -> Option<(&'a mut BlockTree<Block>, u32)> {
        fn find_mut_helper<'a, Block: ChainBlock>(
            block_tree: &'a mut BlockTree<Block>,
            blockhash: &BlockHash,
            depth: u32,
        ) -> Option<(&'a mut BlockTree<Block>, u32)> {
            if block_tree.root.block_hash() == blockhash {
                return Some((block_tree, depth));
            }

            for child in block_tree.children.iter_mut() {
                if let res @ Some(_) = find_mut_helper(child, blockhash, depth + 1) {
                    return res;
                }
            }

            None
        }

        find_mut_helper(self, blockhash, 0)
    }

    /// Returns a `BlockTree` where the hash of the root matches the hash of the provided `block`
    /// if it exists, and `None` otherwise.
    fn find(&self, block_hash: &BlockHash) -> Option<&BlockTree<Block>> {
        if self.root.block_hash() == block_hash {
            return Some(self);
        }

        for child in self.children.iter() {
            if let res @ Some(_) = child.find(block_hash) {
                return res;
            }
        }

        None
    }

    /// Returns the hashes of all blocks in the tree.
    pub fn get_hashes(&self) -> Vec<BlockHash> {
        let mut hashes = Vec::with_capacity(self.children.len() + 1);
        hashes.push(*self.root.block_hash());
        hashes.extend(self.children.iter().flat_map(|child| child.get_hashes()));
        hashes
    }
}

/// An error thrown when trying to add a block that isn't a successor
/// of any block in the tree.
#[derive(Debug)]
pub struct BlockDoesNotExtendTree(pub BlockHash);

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{BlockBuilder, BlockChainBuilder};
    use proptest::collection::vec as pvec;
    use proptest::prelude::*;
    use std::collections::BTreeSet;
    use test_strategy::proptest;

    type BlockTree = super::BlockTree<Block>;

    // For generating arbitrary BlockTrees.
    impl Arbitrary for BlockTree {
        type Parameters = Option<()>;
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            fn build_block_tree(tree: &mut BlockTree, num_children: &[u8]) {
                // Add children.
                if num_children.is_empty() {
                    return;
                }

                for _ in 0..num_children[0] {
                    let mut subtree =
                        BlockTree::new(BlockBuilder::with_prev_header(tree.root.header()).build());

                    build_block_tree(&mut subtree, &num_children[1..]);
                    tree.children.push(subtree);
                }
            }

            // Each depth can have up to 3 children, up to a depth of 10.
            pvec(1..3u8, 0..10)
                .prop_map(|num_children| {
                    let mut tree = BlockTree::new(BlockBuilder::genesis().build());
                    build_block_tree(&mut tree, &num_children);
                    tree
                })
                .boxed()
        }
    }

    #[test]
    fn tree_single_block() {
        let block_tree = BlockTree::new(BlockBuilder::genesis().build());
        let expected_chain = BlockChain {
            first: &block_tree.root,
            successors: vec![],
        };

        assert_eq!(block_tree.blockchains(), vec![expected_chain.clone()]);
        assert_eq!(
            Some((expected_chain, vec![])),
            block_tree.get_chain_with_tip(block_tree.root.block_hash())
        );
    }

    #[test]
    fn tree_multiple_forks() {
        let genesis_block = BlockBuilder::genesis().build();
        let genesis_block_header = *genesis_block.header();
        let mut block_tree = BlockTree::new(genesis_block);

        let mut children = vec![];
        for i in 1..5 {
            // Create different blocks extending the genesis block.
            // Each one of these should be a separate fork.
            let block = BlockBuilder::with_prev_header(&genesis_block_header).build();
            children.push(block.clone());
            block_tree.extend(block).unwrap();
            assert_eq!(block_tree.blockchains().len(), i);
        }

        assert_eq!(block_tree.children.len(), 4);
        assert_eq!(
            Some((
                BlockChain {
                    first: &block_tree.root,
                    successors: vec![],
                },
                children.iter().collect()
            )),
            block_tree.get_chain_with_tip(block_tree.root.block_hash())
        );
    }

    #[test]
    fn chain_with_tip_no_forks() {
        let mut blocks = vec![BlockBuilder::genesis().build()];
        for i in 1..10 {
            blocks.push(BlockBuilder::with_prev_header(blocks[i - 1].header()).build())
        }

        let mut block_tree = BlockTree::new(blocks[0].clone());

        for block in blocks.iter().skip(1) {
            block_tree.extend(block.clone()).unwrap();
        }

        for (i, block) in blocks.iter().enumerate() {
            // Fetch the blockchain with the `block` as tip.
            let block_hash = block.block_hash();
            let chain = block_tree
                .get_chain_with_tip(block_hash)
                .unwrap()
                .0
                .into_chain();

            // The first block should be the genesis block.
            assert_eq!(chain[0], &blocks[0]);
            // The last block should be the expected tip.
            assert_eq!(chain.last().unwrap(), &block);

            // The length of the chain should grow as the requested tip gets deeper.
            assert_eq!(chain.len(), i + 1);

            // All blocks should be correctly chained to one another.
            for i in 1..chain.len() {
                assert_eq!(chain[i - 1].block_hash(), &chain[i].prev_block_hash())
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

            for block in blocks.iter().skip(1) {
                block_tree.extend(block.clone()).unwrap();
            }

            for (i, block) in blocks.iter().enumerate() {
                // Fetch the blockchain with the `block` as tip.
                let block_hash = block.block_hash();
                let chain = block_tree
                    .get_chain_with_tip(block_hash)
                    .unwrap()
                    .0
                    .into_chain();

                // The first block should be the genesis block.
                assert_eq!(chain[0], &blocks[0]);
                // The last block should be the expected tip.
                assert_eq!(chain.last().unwrap(), &block);

                // The length of the chain should grow as the requested tip gets deeper.
                assert_eq!(chain.len(), i + 1);

                // All blocks should be correctly chained to one another.
                for i in 1..chain.len() {
                    assert_eq!(chain[i - 1].block_hash(), &chain[i].prev_block_hash())
                }
            }

            blocks = vec![blocks[0].clone()];
        }
    }

    #[test]
    fn test_difficulty_based_depth_single_block() {
        let block_tree = BlockTree::new(BlockBuilder::genesis().build_with_mock_difficulty(5));

        assert_eq!(
            block_tree.difficulty_based_depth(Network::Mainnet),
            DifficultyBasedDepth::new(5)
        );
    }

    #[test]
    fn test_difficulty_based_depth_root_with_children() {
        let genesis_block = BlockBuilder::genesis().build_with_mock_difficulty(5);
        let genesis_block_header = *genesis_block.header();
        let mut block_tree = BlockTree::new(genesis_block);

        for i in 1..11 {
            block_tree
                .extend(
                    BlockBuilder::with_prev_header(&genesis_block_header)
                        .build_with_mock_difficulty(i),
                )
                .unwrap();
        }

        // The maximum sum of block difficulties from the root to a leaf is the sum
        // of the root and child with the greatest difficulty which is 5 + 10 = 15.
        assert_eq!(
            block_tree.difficulty_based_depth(Network::Mainnet),
            DifficultyBasedDepth::new(15)
        );
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
        expected_blocks_with_depths_by_heights[0].push((&chain[0], chain_len as u32));

        for (i, block) in chain.iter().enumerate().skip(1) {
            expected_blocks_with_depths_by_heights[i].push((block, (chain_len - i) as u32));
            block_tree.extend(block.clone()).unwrap();
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
        block_tree.extend(chain[1].clone()).unwrap();
        block_tree.extend(fork[0].clone()).unwrap();
        block_tree.extend(fork[1].clone()).unwrap();

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

    #[test]
    fn deserialize_very_deep_block_tree() {
        let chain = BlockChainBuilder::new(5_000).build();
        let mut tree = BlockTree::new(chain[0].clone());

        for block in chain.into_iter().skip(1) {
            tree.extend(block).unwrap();
        }

        let mut bytes = vec![];
        ciborium::ser::into_writer(&tree, &mut bytes).unwrap();
        let new_tree: BlockTree = ciborium::de::from_reader(&bytes[..]).unwrap();
        assert_eq!(tree, new_tree);
    }

    #[proptest]
    fn serialize_deserialize(tree: BlockTree) {
        let mut bytes = vec![];
        ciborium::ser::into_writer(&tree, &mut bytes).unwrap();
        let new_tree: BlockTree = ciborium::de::from_reader(&bytes[..]).unwrap();
        assert_eq!(tree, new_tree);
    }

    #[proptest]
    fn should_find_chain_from_tip_and_tip_successors(tree: BlockTree, random_index: usize) {
        fn flatten<'a>(tree: &'a BlockTree, flattened_tree: &mut Vec<&'a Block>) {
            flattened_tree.push(&tree.root);

            for child in &tree.children {
                flatten(child, flattened_tree);
            }
        }
        let mut blocks = vec![];
        flatten(&tree, &mut blocks);
        let chosen_block = blocks[random_index % blocks.len()];

        let (chain, tip_children) = tree.get_chain_with_tip(chosen_block.block_hash()).unwrap();
        let tip = tree
            .find(chain.tip().block_hash())
            .expect("BUG: could not find tip");
        prop_assert_eq!(tip.root.block_hash(), chain.tip().block_hash());

        let actual_children: BTreeSet<_> =
            tip_children.into_iter().map(|b| b.block_hash()).collect();
        let expected_children: BTreeSet<_> = tip
            .get_child_blocks()
            .into_iter()
            .map(|b| b.block_hash())
            .collect();
        prop_assert_eq!(expected_children, actual_children);

        let mut chain = chain.into_chain();
        while let Some(tip) = chain.pop() {
            if let Some(prev) = chain.last() {
                prop_assert_eq!(
                    prev.block_hash(),
                    &ic_btc_types::BlockHash::from(tip.header().prev_blockhash)
                );
            }
        }
    }
}
