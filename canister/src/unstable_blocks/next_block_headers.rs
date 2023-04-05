use crate::types::BlockHash;
use bitcoin::BlockHeader;
use ic_btc_interface::Height;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct NextBlockHeaders {
    hash_to_height_and_header: BTreeMap<BlockHash, (Height, BlockHeader)>,
    height_to_hash: BTreeMap<Height, Vec<BlockHash>>,
}

impl NextBlockHeaders {
    pub fn insert(&mut self, block_header: BlockHeader, height: Height) {
        let block_hash = BlockHash::from(block_header.block_hash());
        let hash_vec = self.height_to_hash.entry(height).or_insert_with(Vec::new);

        if !hash_vec.contains(&block_hash) {
            hash_vec.push(block_hash.clone());
        }

        self.hash_to_height_and_header
            .insert(block_hash, (height, block_header));
    }

    pub fn remove(&mut self, block: &BlockHash) {
        if let Some((height, _)) = self.hash_to_height_and_header.remove(block) {
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
                        self.hash_to_height_and_header.remove(hash);
                    }
                }
            }
        }
    }

    pub fn get_max_height(&self) -> Option<Height> {
        self.height_to_hash.iter().last().map(|(height, _)| *height)
    }

    pub fn get_height(&self, hash: &BlockHash) -> Option<&Height> {
        self.hash_to_height_and_header
            .get(hash)
            .map(|(height, _)| height)
    }

    pub fn get_header(&self, hash: &BlockHash) -> Option<&BlockHeader> {
        self.hash_to_height_and_header.get(hash).map(|res| &res.1)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        test_utils::BlockBuilder, types::BlockHash,
        unstable_blocks::next_block_headers::NextBlockHeaders,
    };

    #[test]
    fn test_get_max_height() {
        let mut block_headers: NextBlockHeaders = Default::default();

        assert_eq!(block_headers.get_max_height(), None);
        let header1 = *BlockBuilder::genesis().build().header();
        let hash1 = BlockHash::from(header1.block_hash());
        let header2 = *BlockBuilder::with_prev_header(&header1).build().header();
        let hash2 = BlockHash::from(header2.block_hash());
        block_headers.insert(header1, 5);

        assert_eq!(block_headers.get_max_height(), Some(5));

        block_headers.insert(header2, 7);

        assert_eq!(block_headers.get_max_height(), Some(7));

        block_headers.remove(&hash2);

        assert_eq!(block_headers.get_max_height(), Some(5));

        block_headers.remove(&hash1);

        assert_eq!(block_headers.get_max_height(), None);
    }

    #[test]
    fn test_insert() {
        let mut block_headers: NextBlockHeaders = Default::default();

        assert_eq!(block_headers.get_max_height(), None);
        let header1 = *BlockBuilder::genesis().build().header();
        let hash1 = BlockHash::from(header1.block_hash());
        let header2 = *BlockBuilder::with_prev_header(&header1).build().header();
        let hash2 = BlockHash::from(header2.block_hash());

        block_headers.insert(header1, 5);

        assert_eq!(block_headers.get_max_height(), Some(5));
        assert_eq!(
            block_headers
                .hash_to_height_and_header
                .get(&hash1)
                .unwrap()
                .0,
            5
        );
        assert_eq!(
            *block_headers.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone()]
        );

        // Check that inserting the same element does not
        // create a duplicate.
        block_headers.insert(header1, 5);

        assert_eq!(block_headers.get_max_height(), Some(5));
        assert_eq!(
            block_headers
                .hash_to_height_and_header
                .get(&hash1)
                .unwrap()
                .0,
            5
        );
        assert_eq!(
            *block_headers.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone()]
        );

        block_headers.insert(header2, 5);

        assert_eq!(block_headers.get_max_height(), Some(5));
        assert_eq!(
            block_headers
                .hash_to_height_and_header
                .get(&hash2)
                .unwrap()
                .0,
            5
        );
        assert_eq!(
            *block_headers.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
    }

    #[test]
    pub(crate) fn test_remove() {
        let mut block_headers: NextBlockHeaders = Default::default();

        assert_eq!(block_headers.get_max_height(), None);
        let header1 = *BlockBuilder::genesis().build().header();
        let hash1 = BlockHash::from(header1.block_hash());
        let header2 = *BlockBuilder::with_prev_header(&header1).build().header();
        let hash2 = BlockHash::from(header2.block_hash());
        let header3 = *BlockBuilder::with_prev_header(&header2).build().header();
        let hash3 = BlockHash::from(header3.block_hash());

        block_headers.insert(header1, 5);
        block_headers.insert(header2, 5);
        block_headers.insert(header3, 7);

        assert_eq!(
            *block_headers.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
        assert_eq!(
            *block_headers.height_to_hash.get(&7).unwrap(),
            vec![hash3.clone()]
        );
        assert_eq!(block_headers.height_to_hash.len(), 2);
        assert_eq!(block_headers.hash_to_height_and_header.len(), 3);
        assert_eq!(block_headers.get_max_height(), Some(7));

        block_headers.remove(&hash2);

        assert_eq!(
            *block_headers.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone()]
        );
        assert_eq!(
            *block_headers.height_to_hash.get(&7).unwrap(),
            vec![hash3.clone()]
        );
        assert_eq!(block_headers.height_to_hash.len(), 2);
        assert_eq!(block_headers.hash_to_height_and_header.len(), 2);
        assert_eq!(block_headers.get_max_height(), Some(7));

        block_headers.remove(&hash3);

        assert_eq!(
            *block_headers.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone()]
        );
        assert_eq!(block_headers.height_to_hash.get(&7), None);
        assert_eq!(block_headers.height_to_hash.len(), 1);
        assert_eq!(block_headers.hash_to_height_and_header.len(), 1);
        assert_eq!(block_headers.get_max_height(), Some(5));

        block_headers.remove(&hash1);

        assert_eq!(block_headers.height_to_hash.get(&5), None);
        assert_eq!(block_headers.height_to_hash.len(), 0);
        assert_eq!(block_headers.hash_to_height_and_header.len(), 0);
        assert_eq!(block_headers.get_max_height(), None);
    }

    #[test]
    pub(crate) fn test_remove_block_until_height() {
        let mut block_headers: NextBlockHeaders = Default::default();

        assert_eq!(block_headers.get_max_height(), None);
        let header1 = *BlockBuilder::genesis().build().header();
        let hash1 = BlockHash::from(header1.block_hash());
        let header2 = *BlockBuilder::with_prev_header(&header1).build().header();
        let hash2 = BlockHash::from(header2.block_hash());
        let header3 = *BlockBuilder::with_prev_header(&header2).build().header();
        let hash3 = BlockHash::from(header3.block_hash());
        let header4 = *BlockBuilder::with_prev_header(&header3).build().header();
        let hash4 = BlockHash::from(header4.block_hash());

        block_headers.insert(header1, 5);
        block_headers.insert(header2, 5);
        block_headers.insert(header3, 7);
        block_headers.insert(header4, 9);

        assert_eq!(
            *block_headers.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
        assert_eq!(
            *block_headers.height_to_hash.get(&7).unwrap(),
            vec![hash3.clone()]
        );
        assert_eq!(
            *block_headers.height_to_hash.get(&9).unwrap(),
            vec![hash4.clone()]
        );
        assert_eq!(block_headers.height_to_hash.len(), 3);
        assert_eq!(block_headers.hash_to_height_and_header.len(), 4);
        assert_eq!(block_headers.get_max_height(), Some(9));

        // Noting changes.
        block_headers.remove_until_height(2);
        assert_eq!(
            *block_headers.height_to_hash.get(&5).unwrap(),
            vec![hash1.clone(), hash2.clone()]
        );
        assert_eq!(
            *block_headers.height_to_hash.get(&7).unwrap(),
            vec![hash3.clone()]
        );
        assert_eq!(
            *block_headers.height_to_hash.get(&9).unwrap(),
            vec![hash4.clone()]
        );
        assert_eq!(block_headers.height_to_hash.len(), 3);
        assert_eq!(block_headers.hash_to_height_and_header.len(), 4);
        assert_eq!(block_headers.get_max_height(), Some(9));

        // All blocks on height 5 are removed.
        block_headers.remove_until_height(6);
        assert_eq!(block_headers.height_to_hash.get(&5), None);
        assert_eq!(
            *block_headers.height_to_hash.get(&7).unwrap(),
            vec![hash3.clone()]
        );
        assert_eq!(
            *block_headers.height_to_hash.get(&9).unwrap(),
            vec![hash4.clone()]
        );
        assert_eq!(block_headers.height_to_hash.len(), 2);
        assert_eq!(block_headers.hash_to_height_and_header.len(), 2);
        assert_eq!(block_headers.get_max_height(), Some(9));

        // All blocks are removed.
        block_headers.remove_until_height(9);
        assert_eq!(block_headers.height_to_hash.get(&5), None);
        assert_eq!(block_headers.height_to_hash.get(&7), None);
        assert_eq!(block_headers.height_to_hash.get(&9), None);
        assert_eq!(block_headers.height_to_hash.len(), 0);
        assert_eq!(block_headers.hash_to_height_and_header.len(), 0);
        assert_eq!(block_headers.get_max_height(), None);
    }
}
