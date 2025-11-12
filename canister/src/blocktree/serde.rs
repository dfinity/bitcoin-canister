use super::{Block, BlockTree};
use serde::{
    de::{Deserializer, Error, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Serialize, Serializer,
};
use std::fmt;

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
