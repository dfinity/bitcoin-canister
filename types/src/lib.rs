//! Types used across crates.
//! NOTE: These types are _not_ part of the interface.

use bitcoin::{
    Block as BitcoinBlock, Network as BitcoinNetwork, OutPoint as BitcoinOutPoint, Target,
};
use candid::CandidType;
use ic_btc_interface::{Network, Txid as PublicTxid};
use ic_stable_structures::{BoundedStorable, Storable};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::RefCell, str::FromStr};

// NOTE: If new fields are added, then the implementation of `PartialEq` should be updated.
#[derive(Clone, Debug, Serialize, Deserialize, Eq)]
pub struct Block {
    block: BitcoinBlock,
    transactions: Vec<Transaction>,
    block_hash: RefCell<Option<BlockHash>>,

    #[cfg(feature = "mock_difficulty")]
    pub mock_difficulty: Option<u64>,
}

impl Block {
    pub fn new(block: BitcoinBlock) -> Self {
        Self {
            transactions: block
                .txdata
                .iter()
                .map(|tx| Transaction::new(tx.clone()))
                .collect(),
            block,
            block_hash: RefCell::new(None),
            #[cfg(feature = "mock_difficulty")]
            mock_difficulty: None,
        }
    }

    pub fn header(&self) -> &bitcoin::block::Header {
        &self.block.header
    }

    pub fn block_hash(&self) -> BlockHash {
        self.block_hash
            .borrow_mut()
            .get_or_insert_with(|| BlockHash::from(self.block.block_hash()))
            .clone()
    }

    pub fn txdata(&self) -> &[Transaction] {
        &self.transactions
    }

    pub fn difficulty(&self, network: Network) -> u64 {
        #[cfg(feature = "mock_difficulty")]
        if let Some(difficulty) = self.mock_difficulty {
            return difficulty;
        }

        Self::target_difficulty(network, self.header().target())
    }

    pub fn consensus_encode(&self, buffer: &mut Vec<u8>) -> Result<usize, std::io::Error> {
        use bitcoin::consensus::Encodable;
        self.block
            .consensus_encode(buffer)
            .map_err(|err| err.into())
    }

    // Computes the difficulty given a block's target.
    // The definition here corresponds to what is referred as "bdiff" in
    // https://en.bitcoin.it/wiki/Difficulty
    pub fn target_difficulty(network: Network, target: Target) -> u64 {
        use primitive_types::U256;

        let max_target = ic_btc_validation::max_target(&into_bitcoin_network(network));
        let max_target = U256::from_big_endian(&max_target.to_be_bytes());
        let target = U256::from_big_endian(&target.to_be_bytes());
        (max_target / target).low_u64()
    }

    pub fn internal_bitcoin_block(&self) -> &BitcoinBlock {
        &self.block
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.block == other.block && self.transactions == other.transactions
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq)]
pub struct Transaction {
    tx: bitcoin::Transaction,
    txid: RefCell<Option<Txid>>,
}

impl Transaction {
    pub fn new(tx: bitcoin::Transaction) -> Self {
        Self {
            tx,
            txid: RefCell::new(None),
        }
    }

    pub fn is_coin_base(&self) -> bool {
        self.tx.is_coinbase()
    }

    pub fn input(&self) -> &[bitcoin::TxIn] {
        &self.tx.input
    }

    pub fn output(&self) -> &[bitcoin::TxOut] {
        &self.tx.output
    }

    pub fn vsize(&self) -> usize {
        self.tx.vsize()
    }

    pub fn base_size(&self) -> usize {
        self.tx.base_size()
    }

    pub fn total_size(&self) -> usize {
        self.tx.total_size()
    }

    pub fn txid(&self) -> Txid {
        if self.txid.borrow().is_none() {
            // Compute the txid as it wasn't computed already.
            // `tx.txid()` is an expensive call, so it's useful to cache.
            let txid = Txid::from(self.tx.compute_txid().as_ref());
            self.txid.borrow_mut().replace(txid);
        }

        self.txid.borrow().clone().expect("txid must be available")
    }
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        // Don't include the `txid` field in the comparison, as it's only a cache.
        self.tx == other.tx
    }
}

impl From<Transaction> for bitcoin::Transaction {
    fn from(tx: Transaction) -> Self {
        tx.tx
    }
}

#[derive(Clone, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
pub struct Txid {
    #[serde(with = "serde_bytes")]
    bytes: Vec<u8>,
}

impl From<Vec<u8>> for Txid {
    fn from(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl From<&[u8]> for Txid {
    fn from(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
        }
    }
}

impl FromStr for Txid {
    type Err = String;

    fn from_str(txid: &str) -> Result<Self, Self::Err> {
        use bitcoin::Txid as BitcoinTxid;
        let bytes = BitcoinTxid::from_str(txid).unwrap();
        Ok(Self::from(bytes.as_ref()))
    }
}

impl Txid {
    pub const fn size() -> u32 {
        32
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.bytes
    }
}

impl From<Txid> for PublicTxid {
    fn from(txid: Txid) -> Self {
        Self::try_from(&txid.bytes[..]).expect("bug: txid is not 32 bytes long")
    }
}

impl From<PublicTxid> for Txid {
    fn from(txid: PublicTxid) -> Self {
        Self {
            bytes: txid.as_ref().to_vec(),
        }
    }
}

impl std::fmt::Debug for Txid {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.clone())
    }
}

impl std::fmt::Display for Txid {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut bytes = self.bytes.clone();
        bytes.reverse();
        write!(f, "{}", hex::encode(bytes))
    }
}

// A blob representing a block hash.
#[derive(CandidType, PartialEq, Clone, Ord, PartialOrd, Eq, Serialize, Deserialize, Hash)]
pub struct BlockHash(Vec<u8>);

impl Storable for BlockHash {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Borrowed(self.0.as_slice())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self::from(bytes.to_vec())
    }
}

impl BoundedStorable for BlockHash {
    const MAX_SIZE: u32 = 32;
    const IS_FIXED_SIZE: bool = true;
}

impl BlockHash {
    pub fn to_vec(self) -> Vec<u8> {
        self.0
    }
}

impl From<Vec<u8>> for BlockHash {
    fn from(bytes: Vec<u8>) -> Self {
        assert_eq!(
            bytes.len() as u32,
            Self::MAX_SIZE,
            "BlockHash must {} bytes",
            Self::MAX_SIZE
        );
        Self(bytes)
    }
}

impl From<&[u8]> for BlockHash {
    fn from(bytes: &[u8]) -> Self {
        Self::from(bytes.to_vec())
    }
}

impl From<bitcoin::BlockHash> for BlockHash {
    fn from(block_hash: bitcoin::BlockHash) -> Self {
        Self::from(block_hash.as_ref())
    }
}

impl FromStr for BlockHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(
            bitcoin::BlockHash::from_str(s)
                .map_err(|e| e.to_string())?
                .as_ref(),
        ))
    }
}

impl ToString for BlockHash {
    fn to_string(&self) -> String {
        let mut b = self.0.clone();
        b.reverse();
        hex::encode(b)
    }
}

impl Default for BlockHash {
    fn default() -> Self {
        Self(vec![0; 32])
    }
}

impl std::fmt::Debug for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockHash({})", self.to_string())
    }
}

fn into_bitcoin_network(network: Network) -> BitcoinNetwork {
    match network {
        Network::Mainnet => BitcoinNetwork::Bitcoin,
        Network::Testnet => BitcoinNetwork::Testnet,
        Network::Regtest => BitcoinNetwork::Regtest,
    }
}

/// A reference to a transaction output.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct OutPoint {
    pub txid: Txid,
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: Txid, vout: u32) -> Self {
        Self { txid, vout }
    }

    pub fn null() -> Self {
        (&BitcoinOutPoint::null()).into()
    }

    pub const fn size() -> u32 {
        const OUTPOINT_VOUT_SIZE: u32 = 4; // The size of a transaction's vout.
        Txid::size() + OUTPOINT_VOUT_SIZE
    }
}

impl From<&BitcoinOutPoint> for OutPoint {
    fn from(bitcoin_outpoint: &BitcoinOutPoint) -> Self {
        Self {
            txid: Txid::from(bitcoin_outpoint.txid.as_ref()),
            vout: bitcoin_outpoint.vout,
        }
    }
}

impl From<OutPoint> for bitcoin::OutPoint {
    fn from(outpoint: OutPoint) -> Self {
        use bitcoin::hashes::Hash;

        Self {
            txid: bitcoin::Txid::from_raw_hash(
                Hash::from_slice(outpoint.txid.as_bytes()).expect("txid must be valid"),
            ),
            vout: outpoint.vout,
        }
    }
}

impl Storable for OutPoint {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut v: Vec<u8> = self.txid.clone().to_vec(); // Store the txid (32 bytes)
        v.append(&mut self.vout.to_le_bytes().to_vec()); // Then the vout (4 bytes)

        // An outpoint is always exactly 36 bytes.
        assert_eq!(v.len(), OutPoint::size() as usize);

        std::borrow::Cow::Owned(v)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        assert_eq!(bytes.len(), 36);
        OutPoint {
            txid: Txid::from(bytes[..32].to_vec()),
            vout: u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
        }
    }
}

impl BoundedStorable for OutPoint {
    const MAX_SIZE: u32 = OutPoint::size();
    const IS_FIXED_SIZE: bool = true;
}

#[test]
fn target_difficulty() {
    // Example found in https://en.bitcoin.it/wiki/Difficulty#How_is_difficulty_calculated.3F_What_is_the_difference_between_bdiff_and_pdiff.3F
    assert_eq!(
        Block::target_difficulty(
            Network::Mainnet,
            Target::from_compact(bitcoin::CompactTarget::from_consensus(0x1b0404cb))
        ),
        16_307
    );

    // Mainnet block 768362.
    // Data pulled from https://www.blockchain.com/explorer/blocks/btc/768362
    assert_eq!(
        Block::target_difficulty(
            Network::Mainnet,
            Target::from_compact(bitcoin::CompactTarget::from_consensus(386397584))
        ),
        35_364_065_900_457
    );

    // Mainnet block 700000.
    // Data pulled from https://www.blockchain.com/explorer/blocks/btc/700000
    assert_eq!(
        Block::target_difficulty(
            Network::Mainnet,
            Target::from_compact(bitcoin::CompactTarget::from_consensus(386877668))
        ),
        18_415_156_832_118
    );

    // Testnet block 2412153.
    // Data pulled from https://www.blockchain.com/explorer/blocks/btc-testnet/2412153
    assert_eq!(
        Block::target_difficulty(
            Network::Testnet,
            Target::from_compact(bitcoin::CompactTarget::from_consensus(422681968))
        ),
        86_564_599
    );

    // Testnet block 1500000.
    // Data pulled from https://www.blockchain.com/explorer/blocks/btc-testnet/1500000
    assert_eq!(
        Block::target_difficulty(
            Network::Testnet,
            Target::from_compact(bitcoin::CompactTarget::from_consensus(457142912))
        ),
        1_032
    );
}
