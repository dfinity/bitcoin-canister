use super::BlockTree;
use crate::types::Block;
use serde::{
    de::{Deserializer, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Serialize, Serializer,
};
use std::fmt;

// Serialize a BlockTree by first flattening it into a list.
//
// This flattening is necessary as a recursive data structure can cause a stack
// overflow if the structure is very deep.
impl Serialize for BlockTree {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Flatten a block tree into a list.
        fn flatten(tree: &BlockTree, flattened_tree: &mut Vec<(Block, usize)>) {
            flattened_tree.push((tree.root.clone(), tree.children.len()));

            for child in &tree.children {
                flatten(child, flattened_tree);
            }
        }

        let mut flattened_tree = vec![];
        flatten(self, &mut flattened_tree);

        let mut seq = serializer.serialize_seq(Some(flattened_tree.len()))?;
        for e in flattened_tree {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for BlockTree {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(BlockTreeDeserializer)
    }
}

struct BlockTreeDeserializer;

impl<'de> Visitor<'de> for BlockTreeDeserializer {
    type Value = BlockTree;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A blocktree deserializer.")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        // Unflatten a block tree from a list back into a `BlockTree` struct.
        fn build_tree<'a, A: SeqAccess<'a>>(seq: &mut A) -> BlockTree {
            let (root, num_children) = seq
                .next_element()
                .expect("reading next element must succeed")
                .expect("root must exist");

            let mut block_tree = BlockTree::new(root);
            for _ in 0..num_children {
                block_tree.children.push(build_tree(seq));
            }

            block_tree
        }

        Ok(build_tree(&mut seq))
    }
}
