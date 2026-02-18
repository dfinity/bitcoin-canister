use bitcoin::{
    block::Header, Address as BitcoinAddress, Network as BitcoinNetwork, Script,
    TxOut as BitcoinTxOut,
};
use candid::CandidType;
use datasize::DataSize;
use ic_btc_interface::{
    Address as AddressStr, GetBalanceRequest as PublicGetBalanceRequest,
    GetUtxosRequest as PublicGetUtxosRequest, Height, Network, Satoshi, UtxosFilter,
    UtxosFilterInRequest,
};
use ic_btc_types::{BlockHash, OutPoint, Txid};
use ic_stable_structures::{
    storable::{Blob, Bound as StableStructuresBound},
    Storable as StableStructuresStorable,
};
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
            value: bitcoin_txout.value.to_sat(),
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
            self.tip_block_hash.to_vec(),
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
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(self.0.as_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.as_bytes().to_vec()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(String::from_utf8(bytes.to_vec()).expect("Loading address cannot fail."))
    }

    const BOUND: StableStructuresBound = StableStructuresBound::Bounded {
        max_size: 90,
        is_fixed_size: false,
    };
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AddressUtxo {
    pub address: Address,
    pub height: Height,
    pub outpoint: OutPoint,
}

impl StableStructuresStorable for AddressUtxo {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
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

    fn into_bytes(self) -> Vec<u8> {
        let AddressUtxo {
            address,
            height,
            outpoint,
        } = self;
        vec![
            Address::into_bytes(address),
            Storable::to_bytes(&height),
            OutPoint::into_bytes(outpoint),
        ]
        .into_iter()
        .flatten()
        .collect()
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

    const BOUND: StableStructuresBound = StableStructuresBound::Bounded {
        max_size: Address::BOUND.max_size() + 4 /* height bytes */ + OutPoint::BOUND.max_size(),
        is_fixed_size: false,
    };
}

pub struct AddressUtxoRange {
    start_bound: Blob<{ AddressUtxo::BOUND.max_size() as usize }>,
    end_bound: Blob<{ AddressUtxo::BOUND.max_size() as usize }>,
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

impl RangeBounds<Blob<{ AddressUtxo::BOUND.max_size() as usize }>> for AddressUtxoRange {
    fn start_bound(&self) -> Bound<&Blob<{ AddressUtxo::BOUND.max_size() as usize }>> {
        Bound::Included(&self.start_bound)
    }

    fn end_bound(&self) -> Bound<&Blob<{ AddressUtxo::BOUND.max_size() as usize }>> {
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
#[derive(CandidType, PartialEq, Clone, Debug, Eq, Serialize, Deserialize, Hash, DataSize)]
pub struct BlockHeaderBlob(Vec<u8>);

impl StableStructuresStorable for BlockHeaderBlob {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(self.0.as_slice())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self::from(bytes.to_vec())
    }

    const BOUND: StableStructuresBound = StableStructuresBound::Bounded {
        max_size: 80,
        is_fixed_size: true,
    };
}

impl From<&Header> for BlockHeaderBlob {
    fn from(header: &Header) -> Self {
        use bitcoin::consensus::Encodable;
        let mut block_header_blob = vec![];
        Header::consensus_encode(header, &mut block_header_blob).unwrap();
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
            bytes.len(),
            Self::BOUND.max_size() as usize,
            "Header must be exactly {} bytes",
            Self::BOUND.max_size()
        );
        Self(bytes)
    }
}

impl From<BlockHeaderBlob> for Vec<u8> {
    fn from(block_header: BlockHeaderBlob) -> Vec<u8> {
        block_header.0
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
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DataSize)]
pub enum GetSuccessorsRequest {
    /// A request containing the hashes of blocks we'd like to retrieve succeessors for.
    #[serde(rename = "initial")]
    Initial(GetSuccessorsRequestInitial),

    /// A follow-up request to retrieve the `FollowUp` response associated with the given page.
    #[serde(rename = "follow_up")]
    FollowUp(PageNumber),
}

#[derive(CandidType, Clone, PartialEq, Eq, Serialize, Deserialize, DataSize)]
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
#[derive(CandidType, Clone, Debug, Deserialize, Hash, PartialEq, Eq, Serialize, DataSize)]
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

#[derive(
    CandidType, Clone, Debug, Default, Deserialize, Hash, PartialEq, Eq, Serialize, DataSize,
)]
pub struct GetSuccessorsCompleteResponse {
    pub blocks: Vec<BlockBlob>,
    pub next: Vec<BlockHeaderBlob>,
}

#[derive(
    CandidType, Clone, Debug, Default, Deserialize, Hash, PartialEq, Eq, Serialize, DataSize,
)]
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

/// Error type for address parsing with network validation.
#[derive(Debug, PartialEq, Eq)]
pub enum AddressParseError {
    /// The address string is malformed and cannot be parsed.
    MalformedAddress,
    /// The address is valid but belongs to a different network.
    WrongNetwork { expected: Network },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Ord, PartialOrd)]
pub struct Address(String);

impl Address {
    /// Creates a new address from a bitcoin script.
    pub fn from_script(script: &Script, network: Network) -> Result<Self, InvalidAddress> {
        BitcoinAddress::from_script(script, into_bitcoin_network(network))
            .map(|address| Self(address.to_string()))
            .map_err(|_| InvalidAddress)
    }

    /// Parses an address string and validates it belongs to the expected network.
    pub fn from_str_checked(s: &str, expected_network: Network) -> Result<Self, AddressParseError> {
        let unchecked_address =
            BitcoinAddress::from_str(s).map_err(|_| AddressParseError::MalformedAddress)?;

        let bitcoin_network = into_bitcoin_network(expected_network);

        unchecked_address
            .require_network(bitcoin_network)
            .map(|address| Address(address.to_string()))
            .map_err(|_| AddressParseError::WrongNetwork {
                expected: expected_network,
            })
    }
}

impl From<BitcoinAddress> for Address {
    fn from(address: BitcoinAddress) -> Self {
        Self(address.to_string())
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

pub use ic_btc_interface::BlockchainInfo;

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
        Network::Testnet => BitcoinNetwork::Testnet4,
        Network::Regtest => BitcoinNetwork::Regtest,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitcoin::{hashes::Hash, PubkeyHash};
    use ic_btc_interface::Txid as PublicTxid;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn serialize_address_utxo(
            pubhash: [u8; 20],
            height: Height,
            txidhash: [u8; 32],
            vout: u32,
        ) {
            // Test OutPoint
            let txid = Txid::from(PublicTxid::from(txidhash));
            let outpoint = OutPoint { txid, vout };
            let bytes = outpoint.clone().into_bytes();
            assert_eq!(outpoint.to_bytes().as_ref(), &bytes, "outpoint mismatch");
            assert_eq!(outpoint, OutPoint::from_bytes(bytes.into()));

            // Test AddressUtxo
            let address = Address::from(BitcoinAddress::p2pkh(PubkeyHash::from_byte_array(pubhash), BitcoinNetwork::Bitcoin));
             let utxo = AddressUtxo { address, height, outpoint };
            let bytes = utxo.clone().into_bytes();
            assert_eq!(utxo.to_bytes().as_ref(), &bytes, "utxo mismatch");
            assert_eq!(utxo, AddressUtxo::from_bytes(bytes.into()));
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
            148, 87, 230, 105, 220, 107, 52, 76, 0, 144, 209, 14, 178, 42, 3, 119, 2, 40, 152, 212,
            96, 127, 189, 241, 227, 206, 242, 163, 35, 193, 63, 169,
        ]);

        assert_eq!(
            txid.to_string(),
            "a93fc123a3f2cee3f1bd7f60d498280277032ab20ed190004c346bdc69e65794"
        );
    }

    #[test]
    fn test_address_from_invalid_script() {
        let script = Script::from_bytes(&[
            0, 17, 97, 69, 142, 51, 3, 137, 205, 4, 55, 238, 159, 227, 100, 29, 112, 204, 24,
        ]); // Invalid script

        assert_eq!(
            Address::from_script(script, Network::Testnet),
            Err(InvalidAddress)
        );
    }
}
