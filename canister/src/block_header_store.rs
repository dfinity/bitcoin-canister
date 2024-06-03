use crate::{memory::Memory, types::BlockHeaderBlob};
use bitcoin::consensus::{Decodable, Encodable};
use bitcoin::BlockHeader;
use ic_btc_interface::Height;
use ic_btc_types::{Block, BlockHash};
use ic_stable_structures::StableBTreeMap;
use serde::{Deserialize, Serialize};

/// Stores block headers and indexes them by block hash and height.
#[derive(Serialize, Deserialize)]
pub struct BlockHeaderStore {
    /// A map of a block hash to its corresponding raw block header.
    // NOTE: Stable structures don't need to be serialized.
    #[serde(skip, default = "init_block_headers")]
    pub block_headers: StableBTreeMap<BlockHash, BlockHeaderBlob, Memory>,

    /// A map of a block height to its corresponding block hash.
    // NOTE: Stable structures don't need to be serialized.
    #[serde(skip, default = "init_block_heights")]
    pub block_heights: StableBTreeMap<Height, BlockHash, Memory>,
}

// NOTE: `PartialEq` is only available in tests as it would be impractically
// expensive in production.
#[cfg(test)]
impl PartialEq for BlockHeaderStore {
    fn eq(&self, other: &Self) -> bool {
        use crate::test_utils::is_stable_btreemap_equal;
        is_stable_btreemap_equal(&self.block_headers, &other.block_headers)
            && is_stable_btreemap_equal(&self.block_heights, &other.block_heights)
    }
}

impl BlockHeaderStore {
    pub fn init() -> Self {
        Self {
            block_headers: init_block_headers(),
            block_heights: init_block_heights(),
        }
    }

    /// Inserts a block's header and hash into the store.
    pub fn insert_block(&mut self, block: &Block, height: Height) {
        let block_hash = block.block_hash();
        let mut header_blob = vec![];
        block
            .header()
            .consensus_encode(&mut header_blob)
            .expect("block header must be valid");

        self.insert(block_hash, BlockHeaderBlob::from(header_blob), height);
    }

    /// Inserts a block's header and hash into the store.
    pub fn insert(&mut self, block_hash: BlockHash, header_blob: BlockHeaderBlob, height: Height) {
        self.block_headers.insert(block_hash.clone(), header_blob);
        self.block_heights.insert(height, block_hash);
    }

    pub fn get_with_block_hash(&self, block_hash: &BlockHash) -> Option<BlockHeader> {
        self.block_headers
            .get(block_hash)
            .map(deserialize_block_header)
    }

    pub fn get_with_height(&self, height: u32) -> Option<BlockHeader> {
        self.block_heights.get(&height).map(|block_hash| {
            self.block_headers
                .get(&block_hash)
                .map(deserialize_block_header)
                .expect("block header must exist")
        })
    }

    /// Returns block headers in the range [start_hegiht, end_height].
    pub fn get_block_headers_in_range(&self, start_height: u32, end_height: u32) -> Vec<Vec<u8>> {
        self.block_heights
            .range(start_height..=end_height)
            .map(|(_, block_hash)| self.block_headers.get(&block_hash).unwrap().into())
            .collect()
    }
}

fn deserialize_block_header(block_header_blob: BlockHeaderBlob) -> BlockHeader {
    BlockHeader::consensus_decode(block_header_blob.as_slice())
        .expect("block header decoding must succeed")
}

fn init_block_headers() -> StableBTreeMap<BlockHash, BlockHeaderBlob, Memory> {
    StableBTreeMap::init(crate::memory::get_block_headers_memory())
}

fn init_block_heights() -> StableBTreeMap<u32, BlockHash, Memory> {
    StableBTreeMap::init(crate::memory::get_block_heights_memory())
}

#[cfg(test)]
mod test {
    use bitcoin::consensus::Encodable;
    use proptest::proptest;

    use crate::{block_header_store::BlockHeaderStore, test_utils::BlockBuilder};

    #[test]
    fn test_get_block_headers_in_range() {
        let mut vec_headers = vec![];
        let block_0 = BlockBuilder::genesis().build();
        vec_headers.push(*block_0.header());

        let mut store = BlockHeaderStore::init();
        store.insert_block(&block_0, 0);
        let block_num = 100;

        for i in 1..block_num {
            let block = BlockBuilder::with_prev_header(&vec_headers[i - 1]).build();
            vec_headers.push(*block.header());
            store.insert_block(&block, i as u32);
        }

        proptest!(|(
            start_range in 0..=block_num - 1,
            range_length in 1..=block_num)|{
                let requested_end = start_range + range_length - 1;

                let res = store.get_block_headers_in_range(start_range as u32, requested_end as u32);

                let end_range = std::cmp::min(requested_end, block_num - 1);

                assert_eq!(res.len(), end_range - start_range + 1);

                for i in start_range..=end_range{
                    let mut expected_block_header = vec![];
                    vec_headers[i].consensus_encode(&mut expected_block_header).unwrap();
                    assert_eq!(expected_block_header, res[i - start_range]);
                }
            }
        );
    }
}
