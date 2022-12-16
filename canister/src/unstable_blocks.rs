mod outpoints_cache;
use crate::{
    blocktree::{self, BlockChain, BlockDoesNotExtendTree, BlockTree},
    types::{Address, Block, BlockHash, OutPoint, TxOut},
    UtxoSet,
};
use bitcoin::Network as BitcoinNetwork;
use ic_btc_types::Height;
use outpoints_cache::OutPointsCache;
use serde::{Deserialize, Serialize};

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
    network: BitcoinNetwork,
}

impl UnstableBlocks {
    pub fn new(
        utxos: &UtxoSet,
        stability_threshold: u32,
        anchor: Block,
        network: BitcoinNetwork,
    ) -> Self {
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
}

/// Returns a reference to the `anchor` block iff ∃ a child `C` of `anchor` that is stable.
pub fn peek(blocks: &UnstableBlocks) -> Option<&Block> {
    get_stable_child(blocks).map(|_| &blocks.tree.root)
}

/// Pops the `anchor` block iff ∃ a child `C` of the `anchor` block that
/// is stable. The child `C` becomes the new `anchor` block, and all its
/// siblings are discarded.
pub fn pop(blocks: &mut UnstableBlocks) -> Option<Block> {
    match get_stable_child(blocks) {
        Some(stable_child_idx) => {
            let old_anchor = blocks.tree.root.clone();

            // Replace the unstable block tree with that of the stable child.
            blocks.tree = blocks.tree.children.swap_remove(stable_child_idx);

            // Remove the outpoints of the old anchor from the cache.
            blocks.outpoints_cache.remove(&old_anchor);

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
            .ok_or_else(|| BlockDoesNotExtendTree(block.clone()))?;

    let height = utxos.next_height() + depth + 1;

    // TODO(EXC-1253): Make this whole function atomic.
    // TODO(EXC-1254): Add time-slicing as inserting a block into the outpoints cache can be expensive.
    // TODO(EXC-1256): Do not maintain the OutPointsCache until we're close to the tip.
    // TODO(EXC-1255): Propagate the error here.
    blocks
        .outpoints_cache
        .insert(utxos, &block, height)
        .unwrap();
    blocktree::extend(parent_block_tree, block)
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

pub fn get_blocks(blocks: &UnstableBlocks) -> Vec<&Block> {
    blocktree::blockchains(&blocks.tree)
        .into_iter()
        .flat_map(|bc| bc.into_chain())
        .collect()
}

/// Returns a blockchain starting from the anchor and ending with the `tip`.
///
/// If the `tip` doesn't exist in the tree, `None` is returned.
pub fn get_chain_with_tip<'a, 'b>(
    blocks: &'a UnstableBlocks,
    tip: &'b BlockHash,
) -> Option<BlockChain<'a>> {
    blocktree::get_chain_with_tip(&blocks.tree, tip)
}

// Returns the index of the `anchor`'s stable child if it exists.
fn get_stable_child(blocks: &UnstableBlocks) -> Option<usize> {
    // Compute the normalized weight of all the children.
    let network = blocks.network;
    let mut weights: Vec<_> = blocks
        .tree
        .children
        .iter()
        .enumerate()
        .map(|(idx, child)| (blocktree::get_normalized_weight(child, network), idx))
        .collect();

    // Sort by weight.
    weights.sort_by_key(|(weight, _child_idx)| *weight);

    match weights.last() {
        Some((biggest_weight, child_idx)) => {
            // The child tree with the biggest weight must have a weight >= stability_threshold.
            if *biggest_weight < blocks.stability_threshold as u128 {
                // Need a depth of at least >= stability_threshold
                return None;
            }

            // If there is more than one child, the difference in weight
            // between the child with the biggest weight and all the others must be >= stability_threshold.
            if weights.len() >= 2 {
                if let Some((second_biggest_weight, _)) = weights.get(weights.len() - 2) {
                    if biggest_weight - second_biggest_weight < blocks.stability_threshold as u128 {
                        // Difference must be >= stability_threshold
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
    use crate::{test_utils::BlockBuilder, types::Network};

    #[test]
    fn empty() {
        let anchor = BlockBuilder::genesis().build();
        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(&utxos, 1, anchor, BitcoinNetwork::from(network));
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest), None);
    }

    #[test]
    fn single_chain_same_dfficulties() {
        let block_0 = BlockBuilder::genesis().build().with_mock_dificulty(1);
        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .build()
            .with_mock_dificulty(1);
        let block_2 = BlockBuilder::with_prev_header(block_1.header())
            .build()
            .with_mock_dificulty(1);

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest =
            UnstableBlocks::new(&utxos, 2, block_0.clone(), BitcoinNetwork::from(network));

        push(&mut forest, &utxos, block_1).unwrap();
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest), None);

        push(&mut forest, &utxos, block_2).unwrap();

        // Block 0 (the anchor) now has one stable child (Block 1).
        // Block 0 should be returned when calling `pop`.
        assert_eq!(peek(&forest), Some(&block_0));
        assert_eq!(pop(&mut forest), Some(block_0));

        // Block 1 is now the anchor. It doesn't have stable
        // children yet, so calling `pop` should return `None`.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest), None);
    }

    #[test]
    fn single_chain_various_dfficulties() {
        let block_0 = BlockBuilder::genesis().build().with_mock_dificulty(5);
        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .build()
            .with_mock_dificulty(20);
        let block_2 = BlockBuilder::with_prev_header(block_1.header())
            .build()
            .with_mock_dificulty(10);
        let block_3 = BlockBuilder::with_prev_header(block_2.header())
            .build()
            .with_mock_dificulty(110);

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest =
            UnstableBlocks::new(&utxos, 6, block_0.clone(), BitcoinNetwork::from(network));

        push(&mut forest, &utxos, block_1.clone()).unwrap();
        push(&mut forest, &utxos, block_2).unwrap();
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest), None);

        push(&mut forest, &utxos, block_3).unwrap();
        // Block 0 (the anchor) now has one stable child (Block 1).
        // Block 0 should be returned when calling `pop`.
        assert_eq!(peek(&forest), Some(&block_0));
        assert_eq!(pop(&mut forest), Some(block_0));

        // Block 1 (the anchor) now has one stable child (Block 2).
        // Block 1 should be returned when calling `pop`.
        assert_eq!(peek(&forest), Some(&block_1));
        assert_eq!(pop(&mut forest), Some(block_1));

        // Block 2 is now the anchor. It doesn't have stable
        // children yet, so calling `pop` should return `None`.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest), None);
    }

    #[test]
    fn forks_same_difficulties() {
        let genesis_block = BlockBuilder::genesis().build().with_mock_dificulty(1);
        let block = BlockBuilder::with_prev_header(genesis_block.header())
            .build()
            .with_mock_dificulty(1);
        let forked_block = BlockBuilder::with_prev_header(genesis_block.header())
            .build()
            .with_mock_dificulty(1);

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest = UnstableBlocks::new(
            &utxos,
            2,
            genesis_block.clone(),
            BitcoinNetwork::from(network),
        );

        push(&mut forest, &utxos, block).unwrap();
        push(&mut forest, &utxos, forked_block.clone()).unwrap();

        // Neither forks are 2-stable, so we shouldn't get anything.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest), None);

        // Extend fork2 by another block.
        let block_1 = BlockBuilder::with_prev_header(forked_block.header())
            .build()
            .with_mock_dificulty(1);
        push(&mut forest, &utxos, block_1.clone()).unwrap();

        //Now fork2 has a normalized weight of 2, while fork1 has normalized
        //weight of 1, hence we cannot get a stable child.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest), None);

        // Extend fork2 by another block.
        let block_2 = BlockBuilder::with_prev_header(block_1.header())
            .build()
            .with_mock_dificulty(1);
        push(&mut forest, &utxos, block_2).unwrap();
        //Now fork2 has a normalized weight of 3, while fork1 has normalized
        //weight of 1, hence we can get a stable child.
        assert_eq!(peek(&forest), Some(&genesis_block));
        assert_eq!(pop(&mut forest), Some(genesis_block));
        assert_eq!(forest.tree.root, forked_block);

        //fork2 is still 2-stable, hence we can get a stable child.
        assert_eq!(peek(&forest), Some(&forked_block));
        assert_eq!(pop(&mut forest), Some(forked_block));
        assert_eq!(forest.tree.root, block_1);

        // No stable children for fork2.
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest), None);
    }

    #[test]
    fn insert_in_order() {
        let block_0 = BlockBuilder::genesis().build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header()).build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header()).build();

        let network = Network::Mainnet;
        let utxos = UtxoSet::new(network);
        let mut forest =
            UnstableBlocks::new(&utxos, 0, block_0.clone(), BitcoinNetwork::from(network));
        push(&mut forest, &utxos, block_1.clone()).unwrap();
        push(&mut forest, &utxos, block_2).unwrap();

        assert_eq!(peek(&forest), Some(&block_0));
        assert_eq!(pop(&mut forest), Some(block_0));
        assert_eq!(peek(&forest), Some(&block_1));
        assert_eq!(pop(&mut forest), Some(block_1));
        assert_eq!(peek(&forest), None);
        assert_eq!(pop(&mut forest), None);
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
        let mut forest =
            UnstableBlocks::new(&utxos, 1, block_0.clone(), BitcoinNetwork::from(network));

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
        let mut forest =
            UnstableBlocks::new(&utxos, 1, block_0.clone(), BitcoinNetwork::from(network));

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
        let mut forest =
            UnstableBlocks::new(&utxos, 1, block_0.clone(), BitcoinNetwork::from(network));

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
        let mut forest =
            UnstableBlocks::new(&utxos, 1, block_0.clone(), BitcoinNetwork::from(network));

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
        let mut forest =
            UnstableBlocks::new(&utxos, 1, block_0.clone(), BitcoinNetwork::from(network));

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
        let mut forest =
            UnstableBlocks::new(&utxos, 1, block_0.clone(), BitcoinNetwork::from(network));

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
        let forest = UnstableBlocks::new(&utxos, 1, block_0.clone(), BitcoinNetwork::from(network));

        assert_eq!(get_main_chain(&forest), BlockChain::new(&block_0));
    }
}
