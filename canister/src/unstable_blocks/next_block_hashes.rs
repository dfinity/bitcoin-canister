use crate::types::BlockHash;
use ic_btc_types::Height;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct NextBlockHashes {
    hash_to_height: BTreeMap<BlockHash, Height>,
    height_to_hash: BTreeMap<Height, Vec<BlockHash>>,
}

impl NextBlockHashes {
    pub fn insert(&mut self, block_hash: &BlockHash, height: Height) {
        let hash_vec = self.height_to_hash.entry(height).or_insert_with(Vec::new);

        if !hash_vec.contains(block_hash) {
            hash_vec.push(block_hash.clone());
        }

        self.hash_to_height.insert(block_hash.clone(), height);
    }

    pub fn remove(&mut self, block: &BlockHash) {
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

    pub fn remove_until_height(&mut self, until_height: Height) {
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

    pub fn get_max_height(&self) -> Option<Height> {
        self.height_to_hash.iter().last().map(|(height, _)| *height)
    }

    pub fn get_height(&self, hash: &BlockHash) -> Option<&Height> {
        self.hash_to_height.get(hash)
    }
}

#[cfg(test)]
mod test {
    use crate::{types::BlockHash, unstable_blocks::next_block_hashes::NextBlockHashes};
    use ic_stable_structures::Storable;

    #[test]
    fn test_get_max_height() {
        let mut block_hashes: NextBlockHashes = Default::default();

        assert_eq!(block_hashes.get_max_height(), None);
        let hash1 = BlockHash::from_bytes(vec![1; 32]);
        let hash2 = BlockHash::from_bytes(vec![2; 32]);
        block_hashes.insert(&hash1, 5);

        assert_eq!(block_hashes.get_max_height(), Some(5));

        block_hashes.insert(&hash2, 7);

        assert_eq!(block_hashes.get_max_height(), Some(7));

        block_hashes.remove(&hash2);

        assert_eq!(block_hashes.get_max_height(), Some(5));

        block_hashes.remove(&hash1);

        assert_eq!(block_hashes.get_max_height(), None);
    }

    #[test]
    fn test_insert() {
        let mut block_hashes: NextBlockHashes = Default::default();

        assert_eq!(block_hashes.get_max_height(), None);
        let hash1 = BlockHash::from_bytes(vec![1; 32]);

        block_hashes.insert(&hash1, 5);

        assert_eq!(block_hashes.get_max_height(), Some(5));
        assert_eq!(*block_hashes.hash_to_height.get(&hash1).unwrap(), 5);
        assert_eq!(
            *block_hashes.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone()]
        );

        // Check that inserting the same element does not
        // create a duplicates.
        block_hashes.insert(&hash1, 5);

        assert_eq!(block_hashes.get_max_height(), Some(5));
        assert_eq!(*block_hashes.hash_to_height.get(&hash1).unwrap(), 5);
        assert_eq!(
            *block_hashes.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone()]
        );

        let hash2 = BlockHash::from_bytes(vec![2; 32]);

        block_hashes.insert(&hash2, 5);

        assert_eq!(block_hashes.get_max_height(), Some(5));
        assert_eq!(*block_hashes.hash_to_height.get(&hash2).unwrap(), 5);
        assert_eq!(
            *block_hashes.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
    }

    #[test]
    pub(crate) fn test_remove_block() {
        let mut block_hashes: NextBlockHashes = Default::default();

        assert_eq!(block_hashes.get_max_height(), None);
        let hash1 = BlockHash::from_bytes(vec![1; 32]);
        let hash2 = BlockHash::from_bytes(vec![2; 32]);
        let hash3 = BlockHash::from_bytes(vec![5; 32]);

        block_hashes.insert(&hash1, 5);
        block_hashes.insert(&hash2, 5);
        block_hashes.insert(&hash3, 7);

        assert_eq!(
            *block_hashes.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
        assert_eq!(
            *block_hashes.height_to_hash.get(&7).unwrap(),
            vec![hash3.clone()]
        );
        assert_eq!(block_hashes.height_to_hash.len(), 2);
        assert_eq!(block_hashes.hash_to_height.len(), 3);
        assert_eq!(block_hashes.get_max_height(), Some(7));

        block_hashes.remove(&hash2);

        assert_eq!(
            *block_hashes.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone()]
        );
        assert_eq!(
            *block_hashes.height_to_hash.get(&7).unwrap(),
            vec![hash3.clone()]
        );
        assert_eq!(block_hashes.height_to_hash.len(), 2);
        assert_eq!(block_hashes.hash_to_height.len(), 2);
        assert_eq!(block_hashes.get_max_height(), Some(7));

        block_hashes.remove(&hash3);

        assert_eq!(
            *block_hashes.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone()]
        );
        assert_eq!(block_hashes.height_to_hash.get(&7), None);
        assert_eq!(block_hashes.height_to_hash.len(), 1);
        assert_eq!(block_hashes.hash_to_height.len(), 1);
        assert_eq!(block_hashes.get_max_height(), Some(5));

        block_hashes.remove(&hash1);

        assert_eq!(block_hashes.height_to_hash.get(&5), None);
        assert_eq!(block_hashes.height_to_hash.len(), 0);
        assert_eq!(block_hashes.hash_to_height.len(), 0);
        assert_eq!(block_hashes.get_max_height(), None);
    }

    #[test]
    pub(crate) fn test_remove_block_until_height() {
        let mut block_hashes: NextBlockHashes = Default::default();

        assert_eq!(block_hashes.get_max_height(), None);
        let hash1 = BlockHash::from_bytes(vec![1; 32]);
        let hash2 = BlockHash::from_bytes(vec![2; 32]);
        let hash3 = BlockHash::from_bytes(vec![5; 32]);
        let hash4 = BlockHash::from_bytes(vec![7; 32]);

        block_hashes.insert(&hash1, 5);
        block_hashes.insert(&hash2, 5);
        block_hashes.insert(&hash3, 7);
        block_hashes.insert(&hash4, 9);

        assert_eq!(
            *block_hashes.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
        assert_eq!(
            *block_hashes.height_to_hash.get(&7).unwrap(),
            vec![hash3.clone()]
        );
        assert_eq!(
            *block_hashes.height_to_hash.get(&9).unwrap(),
            vec![hash4.clone()]
        );
        assert_eq!(block_hashes.height_to_hash.len(), 3);
        assert_eq!(block_hashes.hash_to_height.len(), 4);
        assert_eq!(block_hashes.get_max_height(), Some(9));

        // Noting changes.
        block_hashes.remove_until_height(2);
        assert_eq!(
            *block_hashes.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
        assert_eq!(
            *block_hashes.height_to_hash.get(&7).unwrap(),
            vec![hash3.clone()]
        );
        assert_eq!(
            *block_hashes.height_to_hash.get(&9).unwrap(),
            vec![hash4.clone()]
        );
        assert_eq!(block_hashes.height_to_hash.len(), 3);
        assert_eq!(block_hashes.hash_to_height.len(), 4);
        assert_eq!(block_hashes.get_max_height(), Some(9));

        // All blocks on height 5 are removed.
        block_hashes.remove_until_height(6);
        assert_eq!(block_hashes.height_to_hash.get(&5), None);
        assert_eq!(
            *block_hashes.height_to_hash.get(&7).unwrap(),
            vec![hash3.clone()]
        );
        assert_eq!(
            *block_hashes.height_to_hash.get(&9).unwrap(),
            vec![hash4.clone()]
        );
        assert_eq!(block_hashes.height_to_hash.len(), 2);
        assert_eq!(block_hashes.hash_to_height.len(), 2);
        assert_eq!(block_hashes.get_max_height(), Some(9));

        // All blocks are removed.
        block_hashes.remove_until_height(9);
        assert_eq!(block_hashes.height_to_hash.get(&5), None);
        assert_eq!(block_hashes.height_to_hash.get(&7), None);
        assert_eq!(block_hashes.height_to_hash.get(&9), None);
        assert_eq!(block_hashes.height_to_hash.len(), 0);
        assert_eq!(block_hashes.hash_to_height.len(), 0);
        assert_eq!(block_hashes.get_max_height(), None);
    }
}
