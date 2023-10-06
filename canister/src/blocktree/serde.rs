use super::BlockTree;
use ic_btc_types::Block;
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
        fn next<'de, A: SeqAccess<'de>>(seq: &mut A) -> (Block, usize) {
            seq.next_element()
                .expect("reading next element must succeed")
                .expect("block must exist")
        }

        // A stack containing a pointer to a `BlockTree` along with how many children it has.
        let mut stack: Vec<(*mut BlockTree, usize)> = Vec::new();

        // Read the root and add it to the stack.
        let (root, children_len) = next(&mut seq);
        let mut block_tree = BlockTree::new(root);
        stack.push((&mut block_tree, children_len));

        // Read the remaining children and add them to the tree.
        unsafe {
            while let Some((tree, children_len)) = stack.pop() {
                for _ in 0..children_len {
                    let (child, grand_children_len) = next(&mut seq);

                    // Add the child to the tree.
                    let subtree = BlockTree::new(child);
                    (*tree).children.push(subtree);

                    // Add a pointer to the subtree on the stack.
                    let subtree_ptr: *mut BlockTree = (*tree).children.last_mut().unwrap();
                    stack.push((subtree_ptr, grand_children_len));
                }
            }
        }

        Ok(block_tree)
    }
}
