use crate::{
    memory::Memory,
    types::{Block, BlockHash},
};
use bitcoin::consensus::{Decodable, Encodable};
use bitcoin::BlockHeader;
use ic_btc_types::Height;
use ic_stable_structures::StableBTreeMap;
use serde::{Deserialize, Serialize};

/// Stores block headers and indexes them by block hash and height.
#[derive(Serialize, Deserialize)]
pub struct BlockHeaderStore {
    /// A map of a block hash to its corresponding raw block header.
    // NOTE: Stable structures don't need to be serialized.
    #[serde(skip, default = "init_block_headers")]
    pub block_headers: StableBTreeMap<Memory, BlockHash, Vec<u8>>,

    /// A map of a block height to its corresponding block hash.
    // NOTE: Stable structures don't need to be serialized.
    #[serde(skip, default = "init_block_heights")]
    pub block_heights: StableBTreeMap<Memory, Height, BlockHash>,
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

    pub fn insert(&mut self, block: &Block, height: Height) {
        let block_hash = block.block_hash().to_vec();
        let mut header_bytes = vec![];
        block
            .header()
            .consensus_encode(&mut header_bytes)
            .expect("block header must be valid");

        self.block_headers
            .insert(block_hash.clone(), header_bytes)
            .expect("block header insertion must succeed");

        self.block_heights
            .insert(height, block_hash)
            .expect("block height insertion must succeed");
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
}

const BLOCK_HASH_LENGTH_IN_BYTES: u32 = 32;
const BLOCK_HEADER_LENGTH_IN_BYTES: u32 = 80;
const BLOCK_HEIGHT_LENGTH_IN_BYTES: u32 = 4;

fn deserialize_block_header(block_header_bytes: Vec<u8>) -> BlockHeader {
    BlockHeader::consensus_decode(block_header_bytes.as_slice())
        .expect("block header decoding must succeed")
}

fn init_block_headers() -> StableBTreeMap<Memory, BlockHash, Vec<u8>> {
    StableBTreeMap::init(
        crate::memory::get_block_headers_memory(),
        BLOCK_HASH_LENGTH_IN_BYTES,
        BLOCK_HEADER_LENGTH_IN_BYTES,
    )
}

fn init_block_heights() -> StableBTreeMap<Memory, u32, BlockHash> {
    StableBTreeMap::init(
        crate::memory::get_block_heights_memory(),
        BLOCK_HEIGHT_LENGTH_IN_BYTES,
        BLOCK_HASH_LENGTH_IN_BYTES,
    )
}
