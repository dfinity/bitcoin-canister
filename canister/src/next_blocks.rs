use crate::types::BlockHash;
use ic_btc_types::Height;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct NextBlocks {
    hash_to_height: BTreeMap<BlockHash, Height>,
    height_to_hash: BTreeMap<Height, Vec<BlockHash>>,
}

impl NextBlocks {
    pub(crate) fn insert(&mut self, block_hash: &BlockHash, height: Height) {
        let hash_vec = self.height_to_hash.entry(height).or_insert_with(Vec::new);

        if !hash_vec.contains(block_hash) {
            hash_vec.push(block_hash.clone());
        }

        self.hash_to_height.insert(block_hash.clone(), height);
    }

    pub(crate) fn remove_block(&mut self, block: &BlockHash) {
        if let Some(height) = self.hash_to_height.remove(block) {
            let hash_vec = self.height_to_hash.get_mut(&height).unwrap();
            if hash_vec.len() == 1 {
                self.height_to_hash.remove(&height);
            } else {
                let index = hash_vec.iter().position(|x| *x == *block).unwrap();
                hash_vec.remove(index);
            }
        }
    }

    pub(crate) fn remove_until_height(&mut self, until_height: Height) {
        if let Some((smallest_height, _)) = self.height_to_hash.iter().next() {
            for height in *smallest_height..until_height + 1 {
                if let Some(hash_vec) = self.height_to_hash.remove(&height) {
                    for hash in hash_vec.iter() {
                        self.hash_to_height.remove(hash);
                    }
                }
            }
        }
    }

    pub(crate) fn get_max_height(&self) -> Option<Height> {
        self.height_to_hash
            .iter()
            .next_back()
            .map(|(height, _)| *height)
    }

    pub(crate) fn get_height(&self, hash: &BlockHash) -> Option<&Height> {
        self.hash_to_height.get(hash)
    }
}

#[cfg(test)]
mod test {
    use crate::{next_blocks::NextBlocks, types::BlockHash};
    use ic_stable_structures::Storable;

    #[test]
    fn test_get_max_height() {
        let mut blocks: NextBlocks = Default::default();

        assert_eq!(blocks.get_max_height(), None);
        let hash1 = BlockHash::from_bytes(vec![1; 32]);
        let hash2 = BlockHash::from_bytes(vec![2; 32]);
        blocks.insert(&hash1, 5);

        assert_eq!(blocks.get_max_height(), Some(5));

        blocks.insert(&hash2, 7);

        assert_eq!(blocks.get_max_height(), Some(7));

        blocks.remove_block(&hash2);

        assert_eq!(blocks.get_max_height(), Some(5));

        blocks.remove_block(&hash1);

        assert_eq!(blocks.get_max_height(), None);
    }

    #[test]
    fn test_insert() {
        let mut blocks: NextBlocks = Default::default();

        assert_eq!(blocks.get_max_height(), None);
        let hash1 = BlockHash::from_bytes(vec![1; 32]);

        blocks.insert(&hash1, 5);

        assert_eq!(blocks.get_max_height(), Some(5));
        assert_eq!(*blocks.hash_to_height.get(&hash1).unwrap(), 5);
        assert_eq!(*blocks.height_to_hash.get(&5).unwrap(), vec![hash1.clone()]);

        // Check that insertion the same element does not
        // create a duplicates.
        blocks.insert(&hash1, 5);

        assert_eq!(blocks.get_max_height(), Some(5));
        assert_eq!(*blocks.hash_to_height.get(&hash1).unwrap(), 5);
        assert_eq!(*blocks.height_to_hash.get(&5).unwrap(), vec![hash1.clone()]);

        let hash2 = BlockHash::from_bytes(vec![2; 32]);

        blocks.insert(&hash2, 5);

        assert_eq!(blocks.get_max_height(), Some(5));
        assert_eq!(*blocks.hash_to_height.get(&hash2).unwrap(), 5);
        assert_eq!(
            *blocks.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
    }

    #[test]
    pub(crate) fn test_remove_block() {
        let mut blocks: NextBlocks = Default::default();

        assert_eq!(blocks.get_max_height(), None);
        let hash1 = BlockHash::from_bytes(vec![1; 32]);
        let hash2 = BlockHash::from_bytes(vec![2; 32]);
        let hash3 = BlockHash::from_bytes(vec![5; 32]);

        blocks.insert(&hash1, 5);
        blocks.insert(&hash2, 5);
        blocks.insert(&hash3, 7);

        assert_eq!(
            *blocks.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
        assert_eq!(*blocks.height_to_hash.get(&7).unwrap(), vec![hash3.clone()]);
        assert_eq!(blocks.height_to_hash.len(), 2);
        assert_eq!(blocks.hash_to_height.len(), 3);
        assert_eq!(blocks.get_max_height(), Some(7));

        blocks.remove_block(&hash2);

        assert_eq!(*blocks.height_to_hash.get(&5).unwrap(), vec![hash1.clone()]);
        assert_eq!(*blocks.height_to_hash.get(&7).unwrap(), vec![hash3.clone()]);
        assert_eq!(blocks.height_to_hash.len(), 2);
        assert_eq!(blocks.hash_to_height.len(), 2);
        assert_eq!(blocks.get_max_height(), Some(7));

        blocks.remove_block(&hash3);

        assert_eq!(*blocks.height_to_hash.get(&5).unwrap(), vec![hash1.clone()]);
        assert_eq!(blocks.height_to_hash.get(&7), None);
        assert_eq!(blocks.height_to_hash.len(), 1);
        assert_eq!(blocks.hash_to_height.len(), 1);
        assert_eq!(blocks.get_max_height(), Some(5));

        blocks.remove_block(&hash1);

        assert_eq!(blocks.height_to_hash.get(&5), None);
        assert_eq!(blocks.height_to_hash.len(), 0);
        assert_eq!(blocks.hash_to_height.len(), 0);
        assert_eq!(blocks.get_max_height(), None);
    }

    #[test]
    pub(crate) fn test_remove_block_until_height() {
        let mut blocks: NextBlocks = Default::default();

        assert_eq!(blocks.get_max_height(), None);
        let hash1 = BlockHash::from_bytes(vec![1; 32]);
        let hash2 = BlockHash::from_bytes(vec![2; 32]);
        let hash3 = BlockHash::from_bytes(vec![5; 32]);
        let hash4 = BlockHash::from_bytes(vec![7; 32]);

        blocks.insert(&hash1, 5);
        blocks.insert(&hash2, 5);
        blocks.insert(&hash3, 7);
        blocks.insert(&hash4, 9);

        assert_eq!(
            *blocks.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
        assert_eq!(*blocks.height_to_hash.get(&7).unwrap(), vec![hash3.clone()]);
        assert_eq!(*blocks.height_to_hash.get(&9).unwrap(), vec![hash4.clone()]);
        assert_eq!(blocks.height_to_hash.len(), 3);
        assert_eq!(blocks.hash_to_height.len(), 4);
        assert_eq!(blocks.get_max_height(), Some(9));

        // Noting changes.
        blocks.remove_until_height(2);
        assert_eq!(
            *blocks.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
        assert_eq!(*blocks.height_to_hash.get(&7).unwrap(), vec![hash3.clone()]);
        assert_eq!(*blocks.height_to_hash.get(&9).unwrap(), vec![hash4.clone()]);
        assert_eq!(blocks.height_to_hash.len(), 3);
        assert_eq!(blocks.hash_to_height.len(), 4);
        assert_eq!(blocks.get_max_height(), Some(9));

        // All blocks on height 5 are removed.
        blocks.remove_until_height(6);
        assert_eq!(blocks.height_to_hash.get(&5), None);
        assert_eq!(*blocks.height_to_hash.get(&7).unwrap(), vec![hash3.clone()]);
        assert_eq!(*blocks.height_to_hash.get(&9).unwrap(), vec![hash4.clone()]);
        assert_eq!(blocks.height_to_hash.len(), 2);
        assert_eq!(blocks.hash_to_height.len(), 2);
        assert_eq!(blocks.get_max_height(), Some(9));

        // All blocks are removed.
        blocks.remove_until_height(9);
        assert_eq!(blocks.height_to_hash.get(&5), None);
        assert_eq!(blocks.height_to_hash.get(&7), None);
        assert_eq!(blocks.height_to_hash.get(&9), None);
        assert_eq!(blocks.height_to_hash.len(), 0);
        assert_eq!(blocks.hash_to_height.len(), 0);
        assert_eq!(blocks.get_max_height(), None);
    }
}
