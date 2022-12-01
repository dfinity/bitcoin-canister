use crate::state::OUTPOINT_SIZE;
use bitcoin::{
    Address as BitcoinAddress, Block as BitcoinBlock, Network as BitcoinNetwork,
    OutPoint as BitcoinOutPoint, Script, TxOut as BitcoinTxOut,
};
use ic_btc_types::{
    Address as AddressStr, GetBalanceRequest as PublicGetBalanceRequest,
    GetUtxosRequest as PublicGetUtxosRequest, Height, NetworkInRequest, Satoshi, UtxosFilter,
    UtxosFilterInRequest,
};
use ic_cdk::export::{candid::CandidType, Principal};
use ic_stable_structures::{BoundedStorable, Storable as StableStructuresStorable};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::cell::RefCell;
use std::{cmp::Ordering, convert::TryInto, str::FromStr};

// The longest addresses are bech32 addresses, and a bech32 string can be at most 90 chars.
// See https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki
const MAX_ADDRESS_LENGTH: u32 = 90;

// A Bitcoin block header is always 80 bytes. See:
// https://developer.bitcoin.org/reference/block_chain.html#block-headers
const BLOCK_HEADER_LENGTH: u32 = 80;

/// The payload used to initialize the canister.
#[derive(CandidType, Deserialize)]
pub struct Config {
    pub stability_threshold: u128,
    pub network: Network,

    /// The principal from which blocks are retrieved.
    ///
    /// Setting this source to the management canister means that the blocks will be
    /// fetched directly from the replica, and that's what is used in production.
    pub blocks_source: Principal,

    pub syncing: Flag,

    pub fees: Fees,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            stability_threshold: 0,
            network: Network::Regtest,
            blocks_source: Principal::management_canister(),
            syncing: Flag::Enabled,
            fees: Fees::default(),
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct Fees {
    /// The base fee to charge for all `get_utxos` requests.
    pub get_utxos_base: u128,

    /// The number of cycles to charge per 10 instructions.
    pub get_utxos_cycles_per_ten_instructions: u128,

    /// The maximum amount of cycles that can be charged in a `get_utxos` request.
    /// A request must send at least this amount for it to be accepted.
    pub get_utxos_maximum: u128,

    /// The flat fee to charge for a `get_balance` request.
    pub get_balance: u128,

    /// The maximum amount of cycles that can be charged in a `get_balance` request.
    /// A request must send at least this amount for it to be accepted.
    pub get_balance_maximum: u128,

    /// The flat fee to charge for a `get_current_fee_percentiles` request.
    pub get_current_fee_percentiles: u128,

    /// The maximum amount of cycles that can be charged in a `get_current_fee_percentiles` request.
    /// A request must send at least this amount for it to be accepted.
    pub get_current_fee_percentiles_maximum: u128,

    /// The base fee to charge for all `send_transaction` requests.
    pub send_transaction_base: u128,

    /// The number of cycles to charge for each byte in the transaction.
    pub send_transaction_per_byte: u128,
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

    pub fn block_hash(&self) -> BlockHash {
        BlockHash::from(self.block.block_hash())
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
        use bitcoin::hashes::Hash;

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

impl FromStr for Network {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mainnet" => Ok(Network::Mainnet),
            "testnet" => Ok(Network::Testnet),
            "regtest" => Ok(Network::Regtest),
            _ => Err("Bad network".to_string()),
        }
    }
}

impl From<Network> for BitcoinNetwork {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => BitcoinNetwork::Bitcoin,
            Network::Testnet => BitcoinNetwork::Testnet,
            Network::Regtest => BitcoinNetwork::Regtest,
        }
    }
}

impl From<NetworkInRequest> for Network {
    fn from(network: NetworkInRequest) -> Network {
        match network {
            NetworkInRequest::Mainnet | NetworkInRequest::mainnet => Network::Mainnet,
            NetworkInRequest::Testnet | NetworkInRequest::testnet => Network::Testnet,
            NetworkInRequest::Regtest | NetworkInRequest::regtest => Network::Regtest,
        }
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mainnet => {
                write!(f, "mainnet")
            }
            Self::Testnet => {
                write!(f, "testnet")
            }
            Self::Regtest => {
                write!(f, "regtest")
            }
        }
    }
}

/// Used to signal the cut-off point for returning chunked UTXOs results.
pub struct Page {
    pub tip_block_hash: BlockHash,
    pub height: Height,
    pub outpoint: OutPoint,
}

impl Page {
    pub fn to_bytes(&self) -> Vec<u8> {
        vec![
            self.tip_block_hash.clone().to_vec(),
            Storable::to_bytes(&self.height).to_vec(),
            OutPoint::to_bytes(&self.outpoint).to_vec(),
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

        let tip_block_hash = BlockHash::from_bytes(bytes);

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

impl StableStructuresStorable for OutPoint {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut v: Vec<u8> = self.txid.clone().to_vec(); // Store the txid (32 bytes)
        v.append(&mut self.vout.to_le_bytes().to_vec()); // Then the vout (4 bytes)

        // An outpoint is always exactly 36 bytes.
        assert_eq!(v.len(), OUTPOINT_SIZE as usize);

        std::borrow::Cow::Owned(v)
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        assert_eq!(bytes.len(), 36);
        OutPoint {
            txid: Txid::from(bytes[..32].to_vec()),
            vout: u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
        }
    }
}

impl BoundedStorable for OutPoint {
    fn max_size() -> u32 {
        OUTPOINT_SIZE
    }
}

impl Storable for (TxOut, Height) {
    fn to_bytes(&self) -> Vec<u8> {
        vec![
            self.0.value.to_bytes().to_vec(), // Store the value (8 bytes)
            self.0.script_pubkey.clone(),     // Then the script (size varies)
            Storable::to_bytes(&self.1),      // Then the height (4 bytes)
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn from_bytes(mut bytes: Vec<u8>) -> Self {
        let height = <Height as Storable>::from_bytes(bytes.split_off(bytes.len() - 4));
        let script_pubkey = bytes.split_off(8);
        let value = u64::from_bytes(bytes);
        (
            TxOut {
                value,
                script_pubkey,
            },
            height,
        )
    }
}

impl StableStructuresStorable for Address {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Borrowed(self.0.as_bytes())
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        Address(String::from_utf8(bytes).expect("Loading address cannot fail."))
    }
}

impl BoundedStorable for Address {
    fn max_size() -> u32 {
        MAX_ADDRESS_LENGTH
    }
}

#[derive(PartialEq, Eq, Ord, PartialOrd, Debug)]
pub struct AddressUtxo {
    pub address: Address,
    pub height: Height,
    pub outpoint: OutPoint,
}

impl StableStructuresStorable for AddressUtxo {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let bytes = vec![
            Address::to_bytes(&self.address).to_vec(),
            Storable::to_bytes(&self.height),
            OutPoint::to_bytes(&self.outpoint).to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect();

        std::borrow::Cow::Owned(bytes)
    }

    fn from_bytes(mut bytes: Vec<u8>) -> Self {
        let outpoint_bytes = bytes.split_off(bytes.len() - OUTPOINT_SIZE as usize);
        let height_bytes = bytes.split_off(bytes.len() - 4);

        Self {
            address: Address::from_bytes(bytes),
            height: <Height as Storable>::from_bytes(height_bytes),
            outpoint: OutPoint::from_bytes(outpoint_bytes),
        }
    }
}

impl BoundedStorable for AddressUtxo {
    fn max_size() -> u32 {
        Address::max_size() + 4 /* height bytes */ + OutPoint::max_size()
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
        vec![
            Storable::to_bytes(&self.0),
            OutPoint::to_bytes(&self.1).to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn from_bytes(mut bytes: Vec<u8>) -> Self {
        let outpoint_offset = 4;
        let outpoint_bytes = bytes.split_off(outpoint_offset);

        (
            <Height as Storable>::from_bytes(bytes),
            OutPoint::from_bytes(outpoint_bytes),
        )
    }
}

// A blob representing a block in the standard bitcoin format.
pub type BlockBlob = Vec<u8>;

// A blob representing a block header in the standard bitcoin format.
#[derive(CandidType, PartialEq, Clone, Debug, Eq, Serialize, Deserialize, Hash)]
pub struct BlockHeaderBlob(Vec<u8>);

impl StableStructuresStorable for BlockHeaderBlob {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from(bytes)
    }
}

impl BoundedStorable for BlockHeaderBlob {
    fn max_size() -> u32 {
        BLOCK_HEADER_LENGTH
    }
}

impl BlockHeaderBlob {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for BlockHeaderBlob {
    fn from(bytes: Vec<u8>) -> Self {
        assert_eq!(
            bytes.len() as u32,
            Self::max_size(),
            "BlockHeader must {} bytes",
            Self::max_size()
        );
        Self(bytes)
    }
}

// A blob representing a block hash.
#[derive(
    CandidType, PartialEq, Clone, Debug, Ord, PartialOrd, Eq, Serialize, Deserialize, Hash,
)]
pub struct BlockHash(Vec<u8>);

impl StableStructuresStorable for BlockHash {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from(bytes)
    }
}

impl BoundedStorable for BlockHash {
    fn max_size() -> u32 {
        32
    }
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
            Self::max_size(),
            "BlockHash must {} bytes",
            Self::max_size()
        );
        Self(bytes)
    }
}

impl From<bitcoin::BlockHash> for BlockHash {
    fn from(block_hash: bitcoin::BlockHash) -> Self {
        Self(block_hash.to_vec())
    }
}

impl FromStr for BlockHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            bitcoin::BlockHash::from_str(s)
                .map_err(|e| e.to_string())?
                .to_vec(),
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
        let mut bytes = self.bytes.clone();
        bytes.reverse();
        write!(f, "{}", hex::encode(bytes))
    }
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendTransactionInternalRequest {
    pub network: Network,
    #[serde(with = "serde_bytes")]
    pub transaction: Vec<u8>,
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
    pub anchor: BlockHash,
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

    /// The remaining number of follow ups to this response, which can be retrieved
    /// via `FollowUp` requests.
    pub remaining_follow_ups: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidAddress;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Ord, PartialOrd)]
pub struct Address(String);

impl Address {
    /// Creates a new address from a bitcoin script.
    pub fn from_script(script: &Script, network: Network) -> Result<Self, InvalidAddress> {
        let address = BitcoinAddress::from_script(script, network.into()).ok_or(InvalidAddress)?;

        // Due to a bug in the bitcoin crate, it is possible in some extremely rare cases
        // that `Address:from_script` succeeds even if the address is invalid.
        //
        // To get around this bug, we convert the address to a string, and verify that this
        // string is a valid address.
        //
        // See https://github.com/rust-bitcoin/rust-bitcoin/issues/995 for more information.
        let address_str = address.to_string();
        if BitcoinAddress::from_str(&address_str).is_ok() {
            Ok(Self(address_str))
        } else {
            Err(InvalidAddress)
        }
    }
}

impl From<BitcoinAddress> for Address {
    fn from(address: BitcoinAddress) -> Self {
        Self(address.to_string())
    }
}

impl FromStr for Address {
    type Err = InvalidAddress;

    fn from_str(s: &str) -> Result<Self, InvalidAddress> {
        BitcoinAddress::from_str(s)
            .map(|address| Address(address.to_string()))
            .map_err(|_| InvalidAddress)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(CandidType, Debug, Deserialize, PartialEq)]
pub struct GetBalanceRequest {
    pub address: AddressStr,
    pub min_confirmations: Option<u32>,
}

impl From<PublicGetBalanceRequest> for GetBalanceRequest {
    fn from(request: PublicGetBalanceRequest) -> Self {
        Self {
            address: request.address,
            min_confirmations: request.min_confirmations,
        }
    }
}

/// A request for getting the UTXOs for a given address.
#[derive(CandidType, Debug, Deserialize, PartialEq)]
pub struct GetUtxosRequest {
    pub address: AddressStr,
    pub filter: Option<UtxosFilter>,
}

impl From<PublicGetUtxosRequest> for GetUtxosRequest {
    fn from(request: PublicGetUtxosRequest) -> Self {
        Self {
            address: request.address,
            filter: request.filter.map(|f| match f {
                UtxosFilterInRequest::MinConfirmations(min_confirmations)
                | UtxosFilterInRequest::min_confirmations(min_confirmations) => {
                    UtxosFilter::MinConfirmations(min_confirmations)
                }
                UtxosFilterInRequest::Page(page) | UtxosFilterInRequest::page(page) => {
                    UtxosFilter::Page(page)
                }
            }),
        }
    }
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
pub enum Slicing<T, U> {
    Paused(T),
    Done(U),
}

/// An unspent transaction output.
#[derive(Debug, PartialEq, Eq)]
pub struct Utxo {
    pub height: u32,
    pub outpoint: OutPoint,
    pub value: Satoshi,
}

impl Ord for Utxo {
    fn cmp(&self, other: &Self) -> Ordering {
        // Sort by height in descending order.
        match self.height.cmp(&other.height) {
            Ordering::Less => Ordering::Greater,
            Ordering::Greater => Ordering::Less,
            // Then sort by outpoint.
            Ordering::Equal => match self.outpoint.cmp(&other.outpoint) {
                // Then by value.
                Ordering::Equal => self.value.cmp(&other.value),
                other => other,
            },
        }
    }
}

impl PartialOrd for Utxo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(CandidType, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Debug)]
pub enum Flag {
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
}

/// A request to update the canister's config.
#[derive(CandidType, Deserialize, Default)]
pub struct SetConfigRequest {
    pub stability_threshold: Option<u128>,

    /// Whether or not to enable/disable syncing of blocks from the network.
    pub syncing: Option<Flag>,

    /// The fees to charge for the various endpoints.
    pub fees: Option<Fees>,
}

#[test]
fn test_utxo_ordering() {
    let a = Utxo {
        height: 3,
        outpoint: OutPoint {
            txid: Txid::from(vec![]),
            vout: 0,
        },
        value: 123,
    };

    let b = Utxo {
        height: 2,
        outpoint: OutPoint {
            txid: Txid::from(vec![1]),
            vout: 0,
        },
        value: 123,
    };

    let c = Utxo {
        height: 2,
        outpoint: OutPoint {
            txid: Txid::from(vec![1]),
            vout: 0,
        },
        value: 123,
    };

    let d = Utxo {
        height: 2,
        outpoint: OutPoint {
            txid: Txid::from(vec![1]),
            vout: 0,
        },
        value: 124,
    };

    // a < b == c < d
    assert!(a < b);
    assert!(b < d);
    assert!(a < c);
    assert!(c < d);
    assert!(a < d);

    // d > c == b > a
    assert!(d > c);
    assert!(c > a);
    assert!(d > b);
    assert!(b > a);
    assert!(d > a);

    // c == b
    assert!(c == b);
    assert!(c <= b);
    assert!(c >= b);
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

#[test]
fn address_handles_script_edge_case() {
    // A script that isn't valid, but can be successfully converted into an address
    // due to a bug in the bitcoin crate. See:
    // (https://github.com/rust-bitcoin/rust-bitcoin/issues/995)
    //
    // This test verifies that we're protecting ourselves from that case.
    let script = Script::from(vec![
        0, 17, 97, 69, 142, 51, 3, 137, 205, 4, 55, 238, 159, 227, 100, 29, 112, 204, 24,
    ]);

    assert_eq!(
        Address::from_script(&script, Network::Testnet),
        Err(InvalidAddress)
    );
}
