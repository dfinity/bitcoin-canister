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
        #[cfg(feature = "canbench-rs")]
        let _p = canbench_rs::bench_scope("serialize_blocktree");

        // Flatten a block tree into a list.
        fn flatten(tree: &BlockTree, flattened_tree: &mut Vec<(Block, usize)>) {
            flattened_tree.push((tree.root.clone(), tree.children.len()));

            for child in &tree.children {
                flatten(child, flattened_tree);
            }
        }

        let mut flattened_tree = vec![];
        {
            #[cfg(feature = "canbench-rs")]
            let _p = canbench_rs::bench_scope("serialize_blocktree_flatten");
            flatten(self, &mut flattened_tree);
        }

        #[cfg(feature = "canbench-rs")]
        let _p = canbench_rs::bench_scope("serialize_blocktree_serialize_seq");

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
        fn next<'de, A: SeqAccess<'de>>(seq: &mut A) -> Option<(Block, usize)> {
            seq.next_element()
                .expect("reading next element must succeed")
        }

        // A stack containing a `BlockTree` along with how many children remain to be added to it.
        let mut stack: Vec<(BlockTree, usize)> = Vec::new();

        // Read the root and add it to the stack.
        let (root, children_to_add) = next(&mut seq).unwrap();
        stack.push((BlockTree::new(root), children_to_add));

        while let Some((tree, children_to_add)) = stack.pop() {
            if children_to_add == 0 {
                // No more children. Add tree to its parent if it exists.
                match stack.last_mut() {
                    Some(parent) => parent.0.children.push(tree),
                    None => {
                        // There's no parent to this tree. Deserialization is complete.
                        // Assert that there's no more data to deserialize.
                        assert_eq!(next(&mut seq), None);
                        return Ok(tree);
                    }
                }
            } else {
                // Add the tree back to stack, decrementing the number of children that still need
                // to be added.
                stack.push((tree, children_to_add - 1));

                // Add the child to the stack.
                let (child, grand_children_to_add) = next(&mut seq).unwrap();
                stack.push((BlockTree::new(child), grand_children_to_add));
            }
        }

        unreachable!("expected more while deserializing BlockTree");
    }
}
