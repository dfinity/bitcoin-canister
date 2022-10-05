use crate::state::UTXO_KEY_SIZE;
use bitcoin::{
    hashes::Hash, Block as BitcoinBlock, BlockHash as BitcoinBlockHash, Network as BitcoinNetwork,
    OutPoint as BitcoinOutPoint, TxOut as BitcoinTxOut,
};
use ic_btc_types::{Address, Height, UtxosFilter};
use ic_cdk::export::{candid::CandidType, Principal};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::cell::RefCell;
use std::{convert::TryInto, str::FromStr};

/// The payload used to initialize the canister.
#[derive(CandidType, Deserialize)]
pub struct InitPayload {
    pub stability_threshold: u128,
    pub network: Network,

    /// The canister from which blocks are retrieved.
    /// Defaults to the management canister in production and can be overridden
    /// for testing.
    pub blocks_source: Option<Principal>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq)]
pub struct Block {
    block: BitcoinBlock,
    transactions: Vec<Transaction>,
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
        }
    }

    pub fn header(&self) -> &bitcoin::BlockHeader {
        &self.block.header
    }

    pub fn block_hash(&self) -> bitcoin::BlockHash {
        self.block.block_hash()
    }

    pub fn txdata(&self) -> &[Transaction] {
        &self.transactions
    }

    #[cfg(test)]
    pub fn consensus_encode(&self, buffer: &mut Vec<u8>) -> Result<usize, std::io::Error> {
        use bitcoin::consensus::Encodable;
        self.block.consensus_encode(buffer)
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
        self.tx.is_coin_base()
    }

    pub fn input(&self) -> &[bitcoin::TxIn] {
        &self.tx.input
    }

    pub fn output(&self) -> &[bitcoin::TxOut] {
        &self.tx.output
    }

    pub fn size(&self) -> usize {
        self.tx.size()
    }

    pub fn txid(&self) -> Txid {
        if self.txid.borrow().is_none() {
            // Compute the txid as it wasn't computed already.
            // `tx.txid()` is an expensive call, so it's useful to cache.
            let txid = Txid::from(self.tx.txid().to_vec());
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

#[cfg(test)]
impl From<Transaction> for bitcoin::Transaction {
    fn from(tx: Transaction) -> Self {
        tx.tx
    }
}

/// A reference to a transaction output.
//#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct OutPoint {
    pub txid: Txid,
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: Txid, vout: u32) -> Self {
        Self { txid, vout }
    }
}

impl From<&BitcoinOutPoint> for OutPoint {
    fn from(bitcoin_outpoint: &BitcoinOutPoint) -> Self {
        Self {
            txid: Txid::from(bitcoin_outpoint.txid.to_vec()),
            vout: bitcoin_outpoint.vout,
        }
    }
}

#[cfg(test)]
impl From<OutPoint> for bitcoin::OutPoint {
    fn from(outpoint: OutPoint) -> Self {
        Self {
            txid: bitcoin::Txid::from_hash(
                Hash::from_slice(outpoint.txid.as_bytes()).expect("txid must be valid"),
            ),
            vout: outpoint.vout,
        }
    }
}

/// A Bitcoin transaction's output.
#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct TxOut {
    pub value: u64,
    pub script_pubkey: Vec<u8>,
}

impl From<&BitcoinTxOut> for TxOut {
    fn from(bitcoin_txout: &BitcoinTxOut) -> Self {
        Self {
            value: bitcoin_txout.value,
            script_pubkey: bitcoin_txout.script_pubkey.to_bytes(),
        }
    }
}

#[derive(CandidType, Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Network {
    #[serde(rename = "mainnet")]
    Mainnet,
    #[serde(rename = "testnet")]
    Testnet,
    #[serde(rename = "regtest")]
    Regtest,
}

#[allow(clippy::from_over_into)]
impl Into<BitcoinNetwork> for Network {
    fn into(self) -> BitcoinNetwork {
        match self {
            Network::Mainnet => BitcoinNetwork::Bitcoin,
            Network::Testnet => BitcoinNetwork::Testnet,
            Network::Regtest => BitcoinNetwork::Regtest,
        }
    }
}

/// Used to signal the cut-off point for returning chunked UTXOs results.
pub struct Page {
    pub tip_block_hash: BitcoinBlockHash,
    pub height: Height,
    pub outpoint: OutPoint,
}

impl Page {
    pub fn to_bytes(&self) -> Vec<u8> {
        vec![
            self.tip_block_hash.to_vec(),
            self.height.to_bytes(),
            OutPoint::to_bytes(&self.outpoint),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    pub fn from_bytes(mut bytes: Vec<u8>) -> Result<Self, String> {
        // The first 32 bytes represent the encoded `BlockHash`, the next 4 the
        // `Height` and the remaining the encoded `OutPoint`.
        let height_offset = 32;
        let outpoint_offset = 36;
        let outpoint_bytes = bytes.split_off(outpoint_offset);
        let height_bytes = bytes.split_off(height_offset);

        let tip_block_hash = BitcoinBlockHash::from_hash(
            Hash::from_slice(&bytes)
                .map_err(|err| format!("Could not parse tip block hash: {}", err))?,
        );
        // The height is parsed from bytes that are given by the user, so ensure
        // that any errors are handled gracefully instead of using
        // `Height::from_bytes` that can panic.
        let height = u32::from_be_bytes(
            height_bytes
                .into_iter()
                .map(|byte| byte ^ 255)
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|err| format!("Could not parse page height: {:?}", err))?,
        );
        Ok(Page {
            tip_block_hash,
            height,
            outpoint: OutPoint::from_bytes(outpoint_bytes),
        })
    }
}

/// A trait with convencience methods for storing an element into a stable structure.
pub trait Storable {
    fn to_bytes(&self) -> Vec<u8>;

    fn from_bytes(bytes: Vec<u8>) -> Self;
}

impl Storable for OutPoint {
    fn to_bytes(&self) -> Vec<u8> {
        let mut v: Vec<u8> = self.txid.clone().to_vec(); // Store the txid (32 bytes)
        v.append(&mut self.vout.to_le_bytes().to_vec()); // Then the vout (4 bytes)

        // An outpoint is always exactly to the key size (36 bytes).
        assert_eq!(v.len(), UTXO_KEY_SIZE as usize);

        v
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        assert_eq!(bytes.len(), 36);
        OutPoint {
            txid: Txid::from(bytes[..32].to_vec()),
            vout: u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
        }
    }
}

impl Storable for (TxOut, Height) {
    fn to_bytes(&self) -> Vec<u8> {
        vec![
            self.1.to_le_bytes().to_vec(),       // Store the height (4 bytes)
            self.0.value.to_le_bytes().to_vec(), // Then the value (8 bytes)
            self.0.script_pubkey.clone(),        // Then the script (size varies)
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn from_bytes(mut bytes: Vec<u8>) -> Self {
        let height = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        let value = u64::from_le_bytes(bytes[4..12].try_into().unwrap());
        (
            TxOut {
                value,
                script_pubkey: bytes.split_off(12),
            },
            height,
        )
    }
}

impl Storable for Address {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self
            .len()
            .try_into()
            .expect("Address length must be <= 255")];
        bytes.append(&mut self.as_bytes().to_vec());
        bytes
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let address_len = bytes[0] as usize;
        String::from_utf8(bytes[1..address_len + 1].to_vec()).expect("Loading address cannot fail.")
    }
}

impl Storable for (Address, Height, OutPoint) {
    fn to_bytes(&self) -> Vec<u8> {
        vec![
            Address::to_bytes(&self.0),
            self.1.to_bytes(),
            OutPoint::to_bytes(&self.2),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn from_bytes(mut bytes: Vec<u8>) -> Self {
        let address_len = bytes[0] as usize;
        let height_offset = address_len + 1;
        let outpoint_offset = address_len + 5;
        let outpoint_bytes = bytes.split_off(outpoint_offset);
        let height_bytes = bytes.split_off(height_offset);

        (
            Address::from_bytes(bytes),
            Height::from_bytes(height_bytes),
            OutPoint::from_bytes(outpoint_bytes),
        )
    }
}

impl Storable for Height {
    fn to_bytes(&self) -> Vec<u8> {
        // The height is represented as an XOR'ed big endian byte array
        // so that stored entries are sorted in descending height order.
        self.to_be_bytes().iter().map(|byte| byte ^ 255).collect()
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        u32::from_be_bytes(
            bytes
                .into_iter()
                .map(|byte| byte ^ 255)
                .collect::<Vec<_>>()
                .try_into()
                .expect("height_bytes must of length 4"),
        )
    }
}

impl Storable for (Height, OutPoint) {
    fn to_bytes(&self) -> Vec<u8> {
        vec![self.0.to_bytes(), OutPoint::to_bytes(&self.1)]
            .into_iter()
            .flatten()
            .collect()
    }

    fn from_bytes(mut bytes: Vec<u8>) -> Self {
        let outpoint_offset = 4;
        let outpoint_bytes = bytes.split_off(outpoint_offset);

        (
            Height::from_bytes(bytes),
            OutPoint::from_bytes(outpoint_bytes),
        )
    }
}

// A blob representing a block in the standard bitcoin format.
pub type BlockBlob = Vec<u8>;

// A blob representing a block header in the standard bitcoin format.
pub type BlockHeaderBlob = Vec<u8>;

// A blob representing a block hash.
pub type BlockHash = Vec<u8>;

type PageNumber = u8;

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
        let bytes = BitcoinTxid::from_str(txid).unwrap().to_vec();
        Ok(Self::from(bytes))
    }
}

impl Txid {
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.bytes
    }
}

impl std::fmt::Debug for Txid {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.clone())
    }
}

impl std::fmt::Display for Txid {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.clone())
    }
}

/// A request to retrieve more blocks from the Bitcoin network.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GetSuccessorsRequest {
    /// A request containing the hashes of blocks we'd like to retrieve succeessors for.
    #[serde(rename = "initial")]
    Initial(GetSuccessorsRequestInitial),

    /// A follow-up request to retrieve the `FollowUp` response associated with the given page.
    #[serde(rename = "follow_up")]
    FollowUp(PageNumber),
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSuccessorsRequestInitial {
    pub network: Network,
    pub processed_block_hashes: Vec<BlockHash>,
}

/// A response containing new successor blocks from the Bitcoin network.
#[derive(CandidType, Clone, Debug, Deserialize, Hash, PartialEq, Eq, Serialize)]
pub enum GetSuccessorsResponse {
    /// A complete response that doesn't require pagination.
    #[serde(rename = "complete")]
    Complete(GetSuccessorsCompleteResponse),

    /// A partial response that requires `FollowUp` responses to get the rest of it.
    #[serde(rename = "partial")]
    Partial(GetSuccessorsPartialResponse),

    /// A follow-up response containing a blob of bytes to be appended to the partial response.
    #[serde(rename = "follow_up")]
    FollowUp(BlockBlob),
}

#[derive(CandidType, Clone, Debug, Default, Deserialize, Hash, PartialEq, Eq, Serialize)]
pub struct GetSuccessorsCompleteResponse {
    pub blocks: Vec<BlockBlob>,
    pub next: Vec<BlockHeaderBlob>,
}

#[derive(CandidType, Clone, Debug, Default, Deserialize, Hash, PartialEq, Eq, Serialize)]
pub struct GetSuccessorsPartialResponse {
    /// A block that is partial (i.e. the full blob has not been sent).
    pub partial_block: BlockBlob,

    /// Hashes of next block headers.
    pub next: Vec<BlockHeaderBlob>,

    /// The number of pages in this response. The remaining pages need to be retrieved
    /// via `FollowUp` requests/responses.
    pub num_pages: u8,
}

#[derive(CandidType, Debug, Deserialize, PartialEq)]
pub struct GetBalanceRequest {
    pub address: Address,
    pub min_confirmations: Option<u32>,
}

/// A request for getting the UTXOs for a given address.
#[derive(CandidType, Debug, Deserialize, PartialEq)]
pub struct GetUtxosRequest {
    pub address: Address,
    pub filter: Option<UtxosFilter>,
}

type HeaderField = (String, String);

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: ByteBuf,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<HeaderField>,
    pub body: ByteBuf,
}

/// A type used to facilitate time-slicing.
#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum Slicing<T> {
    Paused(T),
    Done,
}

#[test]
fn test_txid_to_string() {
    let txid = Txid::from(vec![
        148, 87, 230, 105, 220, 107, 52, 76, 0, 144, 209, 14, 178, 42, 3, 119, 2, 40, 152, 212, 96,
        127, 189, 241, 227, 206, 242, 163, 35, 193, 63, 169,
    ]);

    assert_eq!(
        txid.to_string(),
        "a93fc123a3f2cee3f1bd7f60d498280277032ab20ed190004c346bdc69e65794"
    );
}
