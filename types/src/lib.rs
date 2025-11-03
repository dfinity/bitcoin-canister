//! Types used across crates.
//! NOTE: These types are _not_ part of the interface.

use bitcoin::{
    block::Header, hashes::Hash, params::Params, Block as BitcoinBlock, Network as BitcoinNetwork,
    OutPoint as BitcoinOutPoint, Target,
};
use candid::CandidType;
use datasize::DataSize;
use ic_btc_interface::{Network, Txid as PublicTxid};
use ic_stable_structures::{storable::Bound, Storable};
use serde::{de::Deserializer, ser::Serializer, Deserialize, Serialize};
use std::{borrow::Cow, fmt, str::FromStr};

/// Wrapper for [OnceCell] that implements [Serialize] and [Deserialize].
#[derive(Clone, Debug, PartialEq, Eq)]
struct OnceCell<T>(std::cell::OnceCell<T>);

impl<T> OnceCell<T> {
    fn new() -> Self {
        Self(std::cell::OnceCell::new())
    }

    fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        self.0.get_or_init(f)
    }
}

impl<T: Serialize> Serialize for OnceCell<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.get().serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for OnceCell<T>
where
    Option<T>: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cell = OnceCell(std::cell::OnceCell::new());
        if let Some(val) = <Option<T>>::deserialize(deserializer)? {
            assert!(cell.0.set(val).is_ok())
        }
        Ok(cell)
    }
}

// NOTE: If new fields are added, then the implementation of `PartialEq` should be updated.
#[derive(Serialize, Deserialize, Clone, Debug, Eq)]
pub struct Block {
    block: BitcoinBlock,
    transactions: Vec<Transaction>,
    block_hash: OnceCell<BlockHash>,

    #[cfg(feature = "mock_difficulty")]
    pub mock_difficulty: Option<u128>,
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
            block_hash: OnceCell::new(),
            #[cfg(feature = "mock_difficulty")]
            mock_difficulty: None,
        }
    }

    pub fn header(&self) -> &Header {
        &self.block.header
    }

    pub fn block_hash(&self) -> &BlockHash {
        self.block_hash
            .get_or_init(|| BlockHash::from(self.block.block_hash()))
    }

    pub fn txdata(&self) -> &[Transaction] {
        &self.transactions
    }

    pub fn difficulty(&self, network: Network) -> u128 {
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
            .map_err(std::io::Error::other)
    }

    // Computes the difficulty given a block's target.
    // The definition here corresponds to what is referred as "bdiff" in
    // https://en.bitcoin.it/wiki/Difficulty
    pub fn target_difficulty(network: Network, target: Target) -> u128 {
        target.difficulty(Params::new(into_bitcoin_network(network)))
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
    txid: OnceCell<Txid>,
}

impl Transaction {
    pub fn new(tx: bitcoin::Transaction) -> Self {
        Self {
            tx,
            txid: OnceCell::new(),
        }
    }

    pub fn is_coinbase(&self) -> bool {
        self.tx.is_coinbase()
    }

    pub fn input(&self) -> &[bitcoin::TxIn] {
        &self.tx.input
    }

    pub fn output(&self) -> &[bitcoin::TxOut] {
        &self.tx.output
    }

    /// Returns the “virtual size” (vsize) of this transaction.
    ///
    /// Virtual transaction size is defined as Transaction weight / 4 (rounded up to the next integer).
    pub fn vsize(&self) -> usize {
        self.tx.vsize()
    }

    /// Returns the total size of this transaction in bytes.
    ///
    /// Total transaction size is the transaction size in bytes serialized as described in BIP144, including base data and witness data.
    pub fn total_size(&self) -> usize {
        self.tx.total_size()
    }

    /// Returns the base size of this transaction in bytes.
    ///
    /// Base transaction size is the size of the transaction serialised with the witness data stripped.
    pub fn base_size(&self) -> usize {
        self.tx.base_size()
    }

    pub fn txid(&self) -> Txid {
        self.txid
            .get_or_init(||
            // Compute the txid as it wasn't computed already.
            // `tx.txid()` is an expensive call, so it's useful to cache.
            Txid::from(
                self.tx
                    .compute_txid()
                    .as_byte_array()
                    .to_vec(),
            ))
            .clone()
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

impl FromStr for Txid {
    type Err = String;

    fn from_str(txid: &str) -> Result<Self, Self::Err> {
        use bitcoin::Txid as BitcoinTxid;
        let bytes = BitcoinTxid::from_str(txid)
            .unwrap()
            .as_byte_array()
            .to_vec();
        Ok(Self::from(bytes))
    }
}

impl Txid {
    pub const fn size() -> u32 {
        32
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
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
#[derive(
    CandidType,
    Clone,
    Copy,
    Default,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Hash,
    DataSize,
)]
pub struct BlockHash([u8; 32]);

impl Storable for BlockHash {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(self.0.as_slice())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.to_vec()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self::from(bytes.to_vec())
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 32,
        is_fixed_size: true,
    };
}

impl BlockHash {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl From<Vec<u8>> for BlockHash {
    fn from(bytes: Vec<u8>) -> Self {
        assert_eq!(
            bytes.len() as u32,
            Self::BOUND.max_size(),
            "BlockHash must be {} bytes",
            Self::BOUND.max_size()
        );
        let mut arr = [0; 32];
        arr.copy_from_slice(&bytes[..32]);
        Self(arr)
    }
}

impl From<bitcoin::BlockHash> for BlockHash {
    fn from(block_hash: bitcoin::BlockHash) -> Self {
        Self(block_hash.to_byte_array())
    }
}

impl FromStr for BlockHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            *bitcoin::BlockHash::from_str(s)
                .map_err(|e| e.to_string())?
                .as_byte_array(),
        ))
    }
}

impl fmt::Display for BlockHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut b = self.0;
        b.reverse();
        write!(f, "{}", hex::encode(b))
    }
}

impl std::fmt::Debug for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockHash({})", self)
    }
}

pub fn into_bitcoin_network(network: Network) -> BitcoinNetwork {
    match network {
        Network::Mainnet => BitcoinNetwork::Bitcoin,
        Network::Testnet => BitcoinNetwork::Testnet4,
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
            txid: Txid::from(bitcoin_outpoint.txid.as_byte_array().to_vec()),
            vout: bitcoin_outpoint.vout,
        }
    }
}

impl From<OutPoint> for bitcoin::OutPoint {
    fn from(outpoint: OutPoint) -> Self {
        Self {
            txid: bitcoin::Txid::from_raw_hash(
                Hash::from_slice(outpoint.txid.as_bytes()).expect("txid must be valid"),
            ),
            vout: outpoint.vout,
        }
    }
}

impl Storable for OutPoint {
    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        let mut v: Vec<u8> = self.txid.as_bytes().to_vec();
        v.append(&mut self.vout.to_le_bytes().to_vec()); // Then the vout (4 bytes)

        // An outpoint is always exactly 36 bytes.
        assert_eq!(v.len(), OutPoint::size() as usize);

        std::borrow::Cow::Owned(v)
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut v: Vec<u8> = self.txid.as_bytes().to_vec(); // Store the txid (32 bytes)
        v.append(&mut self.vout.to_le_bytes().to_vec()); // Then the vout (4 bytes)

        // An outpoint is always exactly 36 bytes.
        assert_eq!(v.len(), OutPoint::size() as usize);

        v
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        assert_eq!(bytes.len(), 36);
        OutPoint {
            txid: Txid::from(bytes[..32].to_vec()),
            vout: u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: OutPoint::size(),
        is_fixed_size: true,
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::proptest;
    use std::cell::RefCell;

    proptest! {
        // OnceCell<T> is introduced to replace the use of RefCell<Option<T>>.
        // In order to ensure this change does not break compatibility during
        // upgrades, the test below checks their serialization formats (in CBOR)
        // remain the same.
        #[test]
        fn serialization_of_ref_cell_equals_once_cell(hash: Option<[u8; 32]>) {
            let blockhash = hash.map(BlockHash);
            let ref_cell = RefCell::new(blockhash);
             let mut ref_bytes = vec![];
            ciborium::ser::into_writer(&ref_cell, &mut ref_bytes).unwrap();

            let once_cell: OnceCell<BlockHash> = OnceCell::new();
            if let Some(hash) = blockhash {
                let _ = once_cell.get_or_init(|| hash);
            }
            let mut once_bytes = vec![];
            ciborium::ser::into_writer(&once_cell, &mut once_bytes).unwrap();

            // assert the serialized bytes are the same
            assert_eq!(ref_bytes, once_bytes);
        }
    }

    #[derive(Serialize)]
    struct VecHash(Vec<u8>);

    proptest! {
        // BlockHash was changed from Vec<u8> to [u8; 32]. The test below
        // checks their serialization formats (in CBOR) remain the same.
        #[test]
        fn serialization_of_fixed_array_equals_vec(hash: [u8; 32]) {
            let fixed_hash = BlockHash(hash);
             let mut fixed_hash_bytes = vec![];
            ciborium::ser::into_writer(&fixed_hash, &mut fixed_hash_bytes).unwrap();

            let vec_hash = VecHash(hash.to_vec());
            let mut vec_hash_bytes = vec![];
            ciborium::ser::into_writer(&vec_hash, &mut vec_hash_bytes).unwrap();

            // assert the serialized bytes are the same
            assert_eq!(fixed_hash_bytes, vec_hash_bytes);
        }
    }

    #[test]
    fn target_difficulty() {
        use bitcoin::CompactTarget;
        // Example found in https://en.bitcoin.it/wiki/Difficulty#How_is_difficulty_calculated.3F_What_is_the_difference_between_bdiff_and_pdiff.3F
        assert_eq!(
            Block::target_difficulty(
                Network::Mainnet,
                Target::from_compact(CompactTarget::from_consensus(0x1b0404cb))
            ),
            16_307
        );

        // Mainnet block 768362.
        // Data pulled from https://www.blockchain.com/explorer/blocks/btc/768362
        assert_eq!(
            Block::target_difficulty(
                Network::Mainnet,
                Target::from_compact(CompactTarget::from_consensus(386397584))
            ),
            35_364_065_900_457
        );

        // Mainnet block 700000.
        // Data pulled from https://www.blockchain.com/explorer/blocks/btc/700000
        assert_eq!(
            Block::target_difficulty(
                Network::Mainnet,
                Target::from_compact(CompactTarget::from_consensus(386877668))
            ),
            18_415_156_832_118
        );

        // Testnet block 2412153.
        // Data pulled from https://www.blockchain.com/explorer/blocks/btc-testnet/2412153
        assert_eq!(
            Block::target_difficulty(
                Network::Testnet,
                Target::from_compact(CompactTarget::from_consensus(422681968))
            ),
            86_564_599
        );

        // Testnet block 1500000.
        // Data pulled from https://www.blockchain.com/explorer/blocks/btc-testnet/1500000
        assert_eq!(
            Block::target_difficulty(
                Network::Testnet,
                Target::from_compact(CompactTarget::from_consensus(457142912))
            ),
            1_032
        );
    }
}
