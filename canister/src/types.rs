//! Types that are private to the crate.
use crate::state::UTXO_KEY_SIZE;
use bitcoin::{
    hashes::Hash, BlockHash as BitcoinBlockHash, Network as BitcoinNetwork,
    OutPoint as BitcoinOutPoint, TxOut as BitcoinTxOut,
};
use ic_btc_types::{Address, Height};
use ic_cdk::export::{candid::CandidType, Principal};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::convert::TryInto;

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

/// A reference to a transaction output.
#[derive(
    CandidType, Clone, Debug, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize,
)]
pub struct OutPoint {
    #[serde(with = "serde_bytes")]
    pub txid: Vec<u8>,
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: Vec<u8>, vout: u32) -> Self {
        Self { txid, vout }
    }
}

impl From<&BitcoinOutPoint> for OutPoint {
    fn from(bitcoin_outpoint: &BitcoinOutPoint) -> Self {
        Self {
            txid: bitcoin_outpoint.txid.to_vec(),
            vout: bitcoin_outpoint.vout,
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
        let mut v: Vec<u8> = self.txid.clone(); // Store the txid (32 bytes)
        v.append(&mut self.vout.to_le_bytes().to_vec()); // Then the vout (4 bytes)

        // An outpoint is always exactly to the key size (36 bytes).
        assert_eq!(v.len(), UTXO_KEY_SIZE as usize);

        v
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        assert_eq!(bytes.len(), 36);
        OutPoint {
            txid: bytes[..32].to_vec(),
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
type BlockBlob = Vec<u8>;

// A blob representing a block header in the standard bitcoin format.
type BlockHeaderBlob = Vec<u8>;

// A blob representing a block hash.
type BlockHash = Vec<u8>;

/// A request to retrieve more blocks from the Bitcoin network.
#[derive(CandidType, Clone, Debug, PartialEq, Eq)]
pub struct GetSuccessorsRequest {
    pub anchor: BlockHash,
    pub processed_block_hashes: Vec<BlockHash>,
}

/// A response containing new successor blocks from the Bitcoin network.
#[derive(CandidType, Clone, Debug, Default, Deserialize, Hash, PartialEq, Eq)]
pub struct GetSuccessorsResponse {
    pub blocks: Vec<BlockBlob>,
    pub next: Vec<BlockHeaderBlob>,
}

#[derive(CandidType, Debug, Deserialize, PartialEq)]
pub struct GetBalanceRequest {
    pub address: Address,
    pub min_confirmations: Option<u32>,
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
