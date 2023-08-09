use bitcoin::{
    Address as BitcoinAddress, Network as BitcoinNetwork, Script, TxOut as BitcoinTxOut,
};
use candid::CandidType;
use ic_btc_interface::{
    Address as AddressStr, GetBalanceRequest as PublicGetBalanceRequest,
    GetUtxosRequest as PublicGetUtxosRequest, Height, Network, Satoshi, UtxosFilter,
    UtxosFilterInRequest,
};
use ic_btc_types::{BlockHash, OutPoint, Txid};
use ic_stable_structures::{storable::Blob, BoundedStorable, Storable as StableStructuresStorable};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::{
    borrow::Cow,
    cmp::Ordering,
    convert::{TryFrom, TryInto},
    ops::{Bound, RangeBounds},
    str::FromStr,
};

// The expected length in bytes of the page.
const EXPECTED_PAGE_LENGTH: usize = 72;

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
        if bytes.len() != EXPECTED_PAGE_LENGTH {
            return Err(format!(
                "Could not parse the page, the length is {}, but the expected length is {}.",
                bytes.len(),
                EXPECTED_PAGE_LENGTH
            ));
        }

        // The first 32 bytes represent the encoded `BlockHash`, the next 4 the
        // `Height` and the remaining the encoded `OutPoint`.
        let height_offset = 32;
        let outpoint_offset = 36;
        let outpoint_bytes = bytes.split_off(outpoint_offset);
        let height_bytes = bytes.split_off(height_offset);

        let tip_block_hash = BlockHash::from(bytes);

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
            outpoint: OutPoint::from_bytes(Cow::Owned(outpoint_bytes)),
        })
    }
}

/// A trait with convencience methods for storing an element into a stable structure.
pub trait Storable {
    fn to_bytes(&self) -> Vec<u8>;

    fn from_bytes(bytes: Vec<u8>) -> Self;
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
        let value = u64::from_bytes(Cow::Owned(bytes));
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
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Borrowed(self.0.as_bytes())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(String::from_utf8(bytes.to_vec()).expect("Loading address cannot fail."))
    }
}

impl BoundedStorable for Address {
    // The longest addresses are bech32 addresses, and a bech32 string can be at most 90 chars.
    // See https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki
    const MAX_SIZE: u32 = 90;
    const IS_FIXED_SIZE: bool = false;
}

#[derive(PartialEq, Eq, Debug, Clone)]
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

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let len = bytes.len();
        Self {
            address: Address::from_bytes(Cow::Borrowed(
                &bytes[0..len - OutPoint::size() as usize - 4],
            )),
            height: <Height as Storable>::from_bytes(
                bytes[len - OutPoint::size() as usize - 4..len - OutPoint::size() as usize]
                    .to_vec(),
            ),
            outpoint: OutPoint::from_bytes(Cow::Borrowed(
                &bytes[len - OutPoint::size() as usize..],
            )),
        }
    }
}

impl BoundedStorable for AddressUtxo {
    const MAX_SIZE: u32 = Address::MAX_SIZE + 4 /* height bytes */ + OutPoint::MAX_SIZE;
    const IS_FIXED_SIZE: bool = false;
}

pub struct AddressUtxoRange {
    start_bound: Blob<{ AddressUtxo::MAX_SIZE as usize }>,
    end_bound: Blob<{ AddressUtxo::MAX_SIZE as usize }>,
}

impl AddressUtxoRange {
    /// Given an address and UTXO, returns a range that matches with all of the address's UTXOs
    /// that are >= the given UTXO.
    ///
    /// The UTXOs are sorted by height in descending order, and then by outpoint.
    pub fn new(address: &Address, utxo: &Option<Utxo>) -> Self {
        let (start_height, start_outpoint) = match utxo {
            Some(utxo) => (utxo.height, utxo.outpoint.clone()),

            // No UTXO specified. Start with the minimum value possible for a height and OutPoint.
            // Heights are sorted in descending order, so u32::MAX is considered its minimum.
            None => (u32::MAX, OutPoint::new(Txid::from(vec![0; 32]), 0)),
        };

        // The end of the range is the maximum value possible for a height and OutPoint.
        // i.e. the range that matches with all UTXOs of that address that are >= the given UTXO.
        // Heights are sorted in descending order, so `0` is considered its minimum.
        let (end_height, end_outpoint) = (0, OutPoint::new(Txid::from(vec![255; 32]), u32::MAX));

        let start_bound = Blob::try_from(
            AddressUtxo {
                address: address.clone(),
                height: start_height,
                outpoint: start_outpoint,
            }
            .to_bytes()
            .as_ref(),
        )
        .unwrap();

        let end_bound = Blob::try_from(
            AddressUtxo {
                address: address.clone(),
                height: end_height,
                outpoint: end_outpoint,
            }
            .to_bytes()
            .as_ref(),
        )
        .unwrap();

        Self {
            start_bound,
            end_bound,
        }
    }
}

impl RangeBounds<Blob<{ AddressUtxo::MAX_SIZE as usize }>> for AddressUtxoRange {
    fn start_bound(&self) -> Bound<&Blob<{ AddressUtxo::MAX_SIZE as usize }>> {
        Bound::Included(&self.start_bound)
    }

    fn end_bound(&self) -> Bound<&Blob<{ AddressUtxo::MAX_SIZE as usize }>> {
        Bound::Included(&self.end_bound)
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
            OutPoint::from_bytes(Cow::Owned(outpoint_bytes)),
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
        Cow::Borrowed(self.0.as_slice())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self::from(bytes.to_vec())
    }
}

impl BoundedStorable for BlockHeaderBlob {
    // A Bitcoin block header is always 80 bytes. See:
    // https://developer.bitcoin.org/reference/block_chain.html#block-headers
    const MAX_SIZE: u32 = 80;
    const IS_FIXED_SIZE: bool = true;
}

impl From<&bitcoin::BlockHeader> for BlockHeaderBlob {
    fn from(header: &bitcoin::BlockHeader) -> Self {
        use bitcoin::consensus::Encodable;
        let mut block_header_blob = vec![];
        bitcoin::BlockHeader::consensus_encode(header, &mut block_header_blob).unwrap();
        Self(block_header_blob)
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
            Self::MAX_SIZE,
            "BlockHeader must {} bytes",
            Self::MAX_SIZE,
        );
        Self(bytes)
    }
}

type PageNumber = u8;

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

#[derive(CandidType, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSuccessorsRequestInitial {
    pub network: Network,
    pub anchor: BlockHash,
    pub processed_block_hashes: Vec<BlockHash>,
}

impl std::fmt::Debug for GetSuccessorsRequestInitial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GetSuccessorsRequestInitial")
            .field("network", &self.network)
            .field("anchor", &self.anchor)
            .field(
                "processed_block_hashes_len",
                &self.processed_block_hashes.len(),
            )
            .field("processed_block_hashes", &self.processed_block_hashes)
            .finish()
    }
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
        let address = BitcoinAddress::from_script(script, into_bitcoin_network(network))
            .ok_or(InvalidAddress)?;

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

pub fn into_bitcoin_network(network: Network) -> BitcoinNetwork {
    match network {
        Network::Mainnet => BitcoinNetwork::Bitcoin,
        Network::Testnet => BitcoinNetwork::Testnet,
        Network::Regtest => BitcoinNetwork::Regtest,
    }
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
