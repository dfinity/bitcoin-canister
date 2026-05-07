use super::{Block, BlockHash, BlockTree, BlocksCache, CachedBlock};
use bitcoin::block::Header;
use ic_btc_interface::Network;
use serde::{
    de::{Deserializer, Error, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Serialize, Serializer,
};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// Serialization helper to flatten a nested tree into a list.
#[derive(Default)]
struct FlattenedTree<T>(Vec<(T, usize)>);

impl<T> FlattenedTree<T> {
    fn map_from<'a, A>(&mut self, tree: &'a BlockTree<A>, f: &(impl Fn(&'a A) -> T + 'a)) {
        self.0.push((f(&tree.root), tree.children.len()));
        for child in &tree.children {
            self.map_from(child, f)
        }
    }
}

impl<T: Serialize> Serialize for FlattenedTree<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let list = &self.0;
        let mut seq = serializer.serialize_seq(Some(list.len()))?;
        for e in list {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}

/// Serialize `BlockTree<Block>` by flattening it into a list of `Block`.
impl Serialize for BlockTree<Block> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut flattened = FlattenedTree(vec![]);
        {
            #[cfg(feature = "canbench-rs")]
            let _p = canbench_rs::bench_scope("serialize_blocktree_flatten");
            flattened.map_from(self, &|block| block.internal_bitcoin_block());
        }
        {
            #[cfg(feature = "canbench-rs")]
            let _p = canbench_rs::bench_scope("serialize_blocktree_serialize_seq");
            flattened.serialize(serializer)
        }
    }
}

/// Serialize `BlockTree<CachedBlock>` by flattening it into a list of
/// `(Header, BlockHash, Difficulty)` pairs.
///
/// Note that the actual block is not serialized here because they are
/// stored separately in a BlocksCache.
impl Serialize for BlockTree<CachedBlock> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut flattened = FlattenedTree(vec![]);
        {
            #[cfg(feature = "canbench-rs")]
            let _p = canbench_rs::bench_scope("serialize_blocktree_flatten");
            flattened.map_from(self, &|block| {
                (&block.header, &block.block_hash, &block.difficulty)
            });
        }
        {
            #[cfg(feature = "canbench-rs")]
            let _p = canbench_rs::bench_scope("serialize_blocktree_serialize_seq");
            flattened.serialize(serializer)
        }
    }
}

/// Deserialization helper has to be a trait because different `BlockTree` requires
/// different visitor type.
trait TreeVisitor<'de, T>: Sized {
    // Each instance must implement its own `next` function.
    fn next<A: SeqAccess<'de>>(&self, seq: &mut A) -> Result<(T, usize), A::Error>;

    // Common routine that helps implementing [Deserialize::visit_seq].
    fn visit_sequence<A: SeqAccess<'de>>(self, mut seq: A) -> Result<BlockTree<T>, A::Error> {
        // A stack containing a `BlockTree` along with how many children remain to be added to it.
        let mut stack: Vec<(BlockTree<T>, usize)> = Vec::new();

        // Read the root and add it to the stack.
        let (root, children_to_add) = self.next(&mut seq)?;
        stack.push((BlockTree::new(root), children_to_add));

        while let Some((tree, children_to_add)) = stack.pop() {
            if children_to_add == 0 {
                // No more children. Add tree to its parent if it exists.
                match stack.last_mut() {
                    Some(parent) => parent.0.children.push(tree),
                    None => {
                        // There's no parent to this tree. Deserialization is complete.
                        // Assert that there's no more data to deserialize.
                        assert!(self.next(&mut seq).is_err());
                        return Ok(tree);
                    }
                }
            } else {
                // Add the tree back to stack, decrementing the number of children that still need
                // to be added.
                stack.push((tree, children_to_add - 1));

                // Add the child to the stack.
                let (child, grand_children_to_add) = self.next(&mut seq)?;
                stack.push((BlockTree::new(child), grand_children_to_add));
            }
        }

        unreachable!("expected more while deserializing BlockTree");
    }
}

/// Visitor for `BlockTree<Block>`
struct BlockTreeVisitor;

impl<'de> TreeVisitor<'de, Block> for BlockTreeVisitor {
    fn next<A: SeqAccess<'de>>(&self, seq: &mut A) -> Result<(Block, usize), A::Error> {
        seq.next_element::<(bitcoin::Block, usize)>()?
            .map(|(block, size)| (Block::new(block), size))
            .ok_or(A::Error::custom("reading next element must succeed"))
    }
}

impl<'de> Visitor<'de> for BlockTreeVisitor {
    type Value = BlockTree<Block>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A blocktree deserializer.")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        self.visit_sequence(seq)
    }
}

impl<'de> Deserialize<'de> for BlockTree<Block> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(BlockTreeVisitor)
    }
}

/// Visitor for `BlockTree<CachedBlock>`
struct CachedBlockTreeVisitor(Rc<RefCell<Box<dyn BlocksCache>>>);

impl<'de> TreeVisitor<'de, CachedBlock> for CachedBlockTreeVisitor {
    fn next<A: SeqAccess<'de>>(&self, seq: &mut A) -> Result<(CachedBlock, usize), A::Error> {
        seq.next_element::<((Header, BlockHash, u128), usize)>()?
            .map(|((header, block_hash, difficulty), size)| {
                let block = CachedBlock {
                    cache: self.0.clone(),
                    block_hash,
                    difficulty,
                    header,
                    fee_rates: None,
                    utxo_delta: 0,
                };
                (block, size)
            })
            .ok_or(A::Error::custom("reading next element must succeed"))
    }
}

impl<'de> Visitor<'de> for CachedBlockTreeVisitor {
    type Value = BlockTree<CachedBlock>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A blocktree deserializer.")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        self.visit_sequence(seq)
    }
}

/// The deserialization of a BlockTree<CachedBlock> would construct all CachedBlock
/// using a dummy cache. This is a hack because it is difficult to pass an existing
/// cache into the deserialization process, especially when the BlockTree is
/// serveral levels down in a nested struct that derives Deserialize. The caller
/// is expected to manually replace the dummy cache with the real cache after
/// deserialization, otherwise any call into the dummy cache will panic.
impl<'de> Deserialize<'de> for BlockTree<CachedBlock> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cache: Rc<RefCell<Box<dyn BlocksCache>>> = Rc::new(RefCell::new(Box::new(DummyCache)));
        deserializer.deserialize_seq(CachedBlockTreeVisitor(cache))
    }
}

/// Dummy implementation of BlocksCache that always panics.
#[derive(Debug)]
struct DummyCache;

impl BlocksCache for DummyCache {
    fn insert(&mut self, _block_hash: BlockHash, _block: Block) -> bool {
        unimplemented!()
    }
    fn remove(&mut self, _block_hash: &BlockHash) -> bool {
        unimplemented!()
    }
    fn get(&self, _block_hash: &BlockHash) -> Option<Block> {
        unimplemented!()
    }
    fn is_empty(&self) -> bool {
        unimplemented!()
    }
    fn len(&self) -> u64 {
        unimplemented!()
    }
    fn network(&self) -> Network {
        unimplemented!()
    }
    fn collect(&self) -> std::collections::BTreeMap<BlockHash, Block> {
        unimplemented!()
    }
}
