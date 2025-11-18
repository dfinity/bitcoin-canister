use ic_doge_interface::Network;
use ic_doge_types::{Block, BlockHash};
use ic_stable_structures::StableBTreeMap;
use std::fmt;

pub trait BlocksCache: std::fmt::Debug {
    /// Insert a block of the given block_hash into the cache.
    /// Return true if the insertion is successful, or false if block_hash already exists in the cache.
    fn insert(&mut self, block_hash: BlockHash, block: Block) -> bool;

    /// Remove the block with the given hash from the cache.
    /// Return true if the removal is successful, or false if it does not exist in the cache.
    fn remove(&mut self, block_hash: &BlockHash) -> bool;

    /// Look up the block of given block_hash in the cache.
    /// Return the block if it exists, or None if it does not.
    fn get(&self, block_hash: &BlockHash) -> Option<Block>;

    /// Return true if the cache is empty, or false otherwise.
    fn is_empty(&self) -> bool;

    /// Return the number of blocks in the cache.
    fn len(&self) -> u64;

    /// Return the network of the blocks in the cache.
    fn network(&self) -> Network;

    /// Return all block hashes and their associated blocks as a BTreeMap.
    fn collect(&self) -> std::collections::BTreeMap<BlockHash, Block>;
}

pub struct BlocksCacheInStableMem {
    pub network: Network,
    map: StableBTreeMap<BlockHash, Vec<u8>, crate::memory::Memory>,
}

impl fmt::Debug for BlocksCacheInStableMem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BlocksCacheInStableMem {{ network = {}, len = {} }}",
            self.network,
            self.map.len()
        )
    }
}

impl BlocksCacheInStableMem {
    pub fn new(network: Network, memory: crate::memory::Memory) -> Self {
        Self {
            network,
            map: StableBTreeMap::init(memory),
        }
    }
}

impl BlocksCache for BlocksCacheInStableMem {
    fn insert(&mut self, block_hash: BlockHash, block: Block) -> bool {
        let mut bytes = Vec::new();
        block.consensus_encode(&mut bytes).unwrap();
        self.map.insert(block_hash, bytes).is_none()
    }
    fn remove(&mut self, block_hash: &BlockHash) -> bool {
        self.map.remove(block_hash).is_some()
    }
    fn get(&self, block_hash: &BlockHash) -> Option<Block> {
        use bitcoin::consensus::Decodable;
        let bytes = self.map.get(block_hash)?;
        let block = bitcoin::dogecoin::Block::consensus_decode(&mut bytes.as_slice()).ok()?;
        Some(Block::new(block))
    }
    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    fn len(&self) -> u64 {
        self.map.len()
    }
    fn network(&self) -> Network {
        self.network
    }
    fn collect(&self) -> std::collections::BTreeMap<BlockHash, Block> {
        self.map
            .iter()
            .map(|entry| {
                use bitcoin::consensus::Decodable;
                let (hash, bytes) = entry.into_pair();
                let block = bitcoin::dogecoin::Block::consensus_decode(&mut bytes.as_slice())
                    .ok()
                    .unwrap();
                (hash, Block::new(block))
            })
            .collect()
    }
}
