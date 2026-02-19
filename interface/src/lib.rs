//! Types used in the interface of the Bitcoin Canister.

use candid::{CandidType, Deserialize, Principal};
use datasize::DataSize;
use serde::Serialize;
use serde_bytes::ByteBuf;
use std::fmt;
use std::str::FromStr;

pub type Address = String;
pub type Satoshi = u64;
pub type MillisatoshiPerByte = u64;
pub type BlockHash = Vec<u8>;
pub type Height = u32;
pub type Page = ByteBuf;
pub type BlockHeader = Vec<u8>;

/// Default stability threshold for the Bitcoin canister.
/// Must not be zero — a value of 0 can make the canister follow wrong branches,
/// get stuck, and require a manual reset.
const DEFAULT_STABILITY_THRESHOLD: u128 = 144; // ~24 hours at 10 min per block

#[derive(CandidType, Clone, Copy, Deserialize, Debug, Eq, PartialEq, Serialize, Hash, DataSize)]
pub enum Network {
    /// Bitcoin Mainnet.
    #[serde(rename = "mainnet")]
    Mainnet,

    /// Bitcoin Testnet4.
    #[serde(rename = "testnet")]
    Testnet,

    /// Bitcoin Regtest.
    #[serde(rename = "regtest")]
    Regtest,
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Mainnet => write!(f, "mainnet"),
            Self::Testnet => write!(f, "testnet"),
            Self::Regtest => write!(f, "regtest"),
        }
    }
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

impl From<Network> for NetworkInRequest {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => Self::Mainnet,
            Network::Testnet => Self::Testnet,
            Network::Regtest => Self::Regtest,
        }
    }
}

impl From<NetworkInRequest> for Network {
    fn from(network: NetworkInRequest) -> Self {
        match network {
            NetworkInRequest::Mainnet => Self::Mainnet,
            NetworkInRequest::mainnet => Self::Mainnet,
            NetworkInRequest::Testnet => Self::Testnet,
            NetworkInRequest::testnet => Self::Testnet,
            NetworkInRequest::Regtest => Self::Regtest,
            NetworkInRequest::regtest => Self::Regtest,
        }
    }
}

/// A network enum that allows both upper and lowercase variants.
/// Supporting both variants allows us to be compatible with the spec (lowercase)
/// while not breaking current dapps that are using uppercase variants.
#[derive(CandidType, Clone, Copy, Deserialize, Debug, Eq, PartialEq, Serialize, Hash)]
pub enum NetworkInRequest {
    /// Bitcoin Mainnet.
    Mainnet,
    /// Bitcoin Mainnet.
    #[allow(non_camel_case_types)]
    mainnet,

    /// Bitcoin Testnet4.
    Testnet,
    /// Bitcoin Testnet4.
    #[allow(non_camel_case_types)]
    testnet,

    /// Bitcoin Regtest.
    Regtest,
    /// Bitcoin Regtest.
    #[allow(non_camel_case_types)]
    regtest,
}

impl fmt::Display for NetworkInRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Mainnet => write!(f, "mainnet"),
            Self::Testnet => write!(f, "testnet"),
            Self::Regtest => write!(f, "regtest"),
            Self::mainnet => write!(f, "mainnet"),
            Self::testnet => write!(f, "testnet"),
            Self::regtest => write!(f, "regtest"),
        }
    }
}

#[derive(CandidType, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Txid([u8; 32]);

impl AsRef<[u8]> for Txid {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Txid> for [u8; 32] {
    fn from(txid: Txid) -> Self {
        txid.0
    }
}

impl serde::Serialize for Txid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> serde::de::Deserialize<'de> for Txid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct TxidVisitor;

        impl<'de> serde::de::Visitor<'de> for TxidVisitor {
            type Value = Txid;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a 32-byte array")
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match TryInto::<[u8; 32]>::try_into(value) {
                    Ok(txid) => Ok(Txid(txid)),
                    Err(_) => Err(E::invalid_length(value.len(), &self)),
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                use serde::de::Error;
                if let Some(size_hint) = seq.size_hint() {
                    if size_hint != 32 {
                        return Err(A::Error::invalid_length(size_hint, &self));
                    }
                }
                let mut bytes = [0u8; 32];
                let mut i = 0;
                while let Some(byte) = seq.next_element()? {
                    if i == 32 {
                        return Err(A::Error::invalid_length(i + 1, &self));
                    }

                    bytes[i] = byte;
                    i += 1;
                }
                if i != 32 {
                    return Err(A::Error::invalid_length(i, &self));
                }
                Ok(Txid(bytes))
            }
        }

        deserializer.deserialize_bytes(TxidVisitor)
    }
}

impl fmt::Display for Txid {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        // In Bitcoin, you display hash bytes in reverse order.
        //
        // > Due to historical accident, the tx and block hashes that bitcoin core
        // > uses are byte-reversed. I’m not entirely sure why. Maybe something
        // > like using openssl bignum to store hashes or something like that,
        // > then printing them as a number.
        // > -- Wladimir van der Laan
        //
        // Source: https://learnmeabitcoin.com/technical/txid
        for b in self.0.iter().rev() {
            write!(fmt, "{:02x}", *b)?
        }
        Ok(())
    }
}

impl From<[u8; 32]> for Txid {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<&'_ [u8]> for Txid {
    type Error = core::array::TryFromSliceError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let txid: [u8; 32] = bytes.try_into()?;
        Ok(Txid(txid))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxidFromStrError {
    InvalidChar(u8),
    InvalidLength { expected: usize, actual: usize },
}

impl fmt::Display for TxidFromStrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidChar(c) => write!(f, "char {c} is not a valid hex"),
            Self::InvalidLength { expected, actual } => write!(
                f,
                "Bitcoin transaction id must be precisely {expected} characters, got {actual}"
            ),
        }
    }
}

impl FromStr for Txid {
    type Err = TxidFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn decode_hex_char(c: u8) -> Result<u8, TxidFromStrError> {
            match c {
                b'A'..=b'F' => Ok(c - b'A' + 10),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                b'0'..=b'9' => Ok(c - b'0'),
                _ => Err(TxidFromStrError::InvalidChar(c)),
            }
        }
        if s.len() != 64 {
            return Err(TxidFromStrError::InvalidLength {
                expected: 64,
                actual: s.len(),
            });
        }
        let mut bytes = [0u8; 32];
        let chars = s.as_bytes();
        for i in 0..32 {
            bytes[31 - i] =
                (decode_hex_char(chars[2 * i])? << 4) | decode_hex_char(chars[2 * i + 1])?;
        }
        Ok(Self(bytes))
    }
}

/// A reference to a transaction output.
#[derive(
    CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct OutPoint {
    /// A cryptographic hash of the transaction.
    /// A transaction can output multiple UTXOs.
    pub txid: Txid,
    /// The index of the output within the transaction.
    pub vout: u32,
}

/// An unspent transaction output.
#[derive(
    CandidType, Debug, Deserialize, Ord, PartialOrd, PartialEq, Serialize, Clone, Hash, Eq,
)]
pub struct Utxo {
    pub outpoint: OutPoint,
    pub value: Satoshi,
    pub height: Height,
}

/// A filter used when requesting UTXOs.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq)]
pub enum UtxosFilter {
    MinConfirmations(u32),
    Page(Page),
}

impl From<UtxosFilterInRequest> for UtxosFilter {
    fn from(filter: UtxosFilterInRequest) -> Self {
        match filter {
            UtxosFilterInRequest::MinConfirmations(x) => Self::MinConfirmations(x),
            UtxosFilterInRequest::min_confirmations(x) => Self::MinConfirmations(x),
            UtxosFilterInRequest::Page(p) => Self::Page(p),
            UtxosFilterInRequest::page(p) => Self::Page(p),
        }
    }
}

/// A UtxosFilter enum that allows both upper and lowercase variants.
/// Supporting both variants allows us to be compatible with the spec (lowercase)
/// while not breaking current dapps that are using uppercase variants.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq)]
pub enum UtxosFilterInRequest {
    MinConfirmations(u32),
    #[allow(non_camel_case_types)]
    min_confirmations(u32),
    Page(Page),
    #[allow(non_camel_case_types)]
    page(Page),
}

/// A request for getting the UTXOs for a given address.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq)]
pub struct GetUtxosRequest {
    pub address: Address,
    pub network: NetworkInRequest,
    pub filter: Option<UtxosFilterInRequest>,
}

/// The response returned for a request to get the UTXOs of a given address.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct GetUtxosResponse {
    pub utxos: Vec<Utxo>,
    pub tip_block_hash: BlockHash,
    pub tip_height: Height,
    pub next_page: Option<Page>,
}

/// Errors when processing a `get_utxos` request.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq, Clone)]
pub enum GetUtxosError {
    MalformedAddress,
    AddressForWrongNetwork { expected: Network },
    MinConfirmationsTooLarge { given: u32, max: u32 },
    UnknownTipBlockHash { tip_block_hash: BlockHash },
    MalformedPage { err: String },
}

/// A request for getting the block headers from a given height.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq)]
pub struct GetBlockHeadersRequest {
    pub start_height: Height,
    pub end_height: Option<Height>,
    pub network: NetworkInRequest,
}

/// The response returned for a request for getting the block headers from a given height.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct GetBlockHeadersResponse {
    pub tip_height: Height,
    pub block_headers: Vec<BlockHeader>,
}

/// Errors when processing a `get_block_headers` request.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq, Clone)]
pub enum GetBlockHeadersError {
    StartHeightDoesNotExist {
        requested: Height,
        chain_height: Height,
    },
    EndHeightDoesNotExist {
        requested: Height,
        chain_height: Height,
    },
    StartHeightLargerThanEndHeight {
        start_height: Height,
        end_height: Height,
    },
}

impl fmt::Display for GetBlockHeadersError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StartHeightDoesNotExist {
                requested,
                chain_height,
            } => {
                write!(
                    f,
                    "The requested start_height is larger than the height of the chain. Requested: {}, height of chain: {}",
                    requested, chain_height
                )
            }
            Self::EndHeightDoesNotExist {
                requested,
                chain_height,
            } => {
                write!(
                    f,
                    "The requested start_height is larger than the height of the chain. Requested: {}, height of chain: {}",
                    requested, chain_height
                )
            }
            Self::StartHeightLargerThanEndHeight {
                start_height,
                end_height,
            } => {
                write!(
                    f,
                    "The requested start_height is larger than the requested end_height. start_height: {}, end_height: {}", start_height, end_height)
            }
        }
    }
}

/// A request for getting the current fee percentiles.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq)]
pub struct GetCurrentFeePercentilesRequest {
    pub network: NetworkInRequest,
}

impl fmt::Display for GetUtxosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedAddress => {
                write!(f, "Malformed address.")
            }
            Self::MinConfirmationsTooLarge { given, max } => {
                write!(
                    f,
                    "The requested min_confirmations is too large. Given: {}, max supported: {}",
                    given, max
                )
            }
            Self::UnknownTipBlockHash { tip_block_hash } => {
                write!(
                    f,
                    "The provided tip block hash {:?} is unknown.",
                    tip_block_hash
                )
            }
            Self::MalformedPage { err } => {
                write!(f, "The provided page is malformed {}", err)
            }
            Self::AddressForWrongNetwork { expected } => {
                write!(
                    f,
                    "Address does not belong to the expected network: {}",
                    expected
                )
            }
        }
    }
}

#[derive(CandidType, Debug, Deserialize, PartialEq, Eq)]
pub struct GetBalanceRequest {
    pub address: Address,
    pub network: NetworkInRequest,
    pub min_confirmations: Option<u32>,
}

#[derive(CandidType, Debug, Deserialize, PartialEq, Eq, Clone)]
pub enum GetBalanceError {
    MalformedAddress,
    AddressForWrongNetwork { expected: Network },
    MinConfirmationsTooLarge { given: u32, max: u32 },
}

impl fmt::Display for GetBalanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedAddress => {
                write!(f, "Malformed address.")
            }
            Self::MinConfirmationsTooLarge { given, max } => {
                write!(
                    f,
                    "The requested min_confirmations is too large. Given: {}, max supported: {}",
                    given, max
                )
            }
            Self::AddressForWrongNetwork { expected } => {
                write!(
                    f,
                    "Address does not belong to the expected network: {}",
                    expected
                )
            }
        }
    }
}

#[derive(CandidType, Debug, Deserialize, PartialEq, Eq)]
pub struct SendTransactionRequest {
    #[serde(with = "serde_bytes")]
    pub transaction: Vec<u8>,
    pub network: NetworkInRequest,
}

#[derive(CandidType, Clone, Debug, Deserialize, PartialEq, Eq)]
pub enum SendTransactionError {
    /// Can't deserialize transaction.
    MalformedTransaction,
    /// Enqueueing a request failed due to full queue to the Bitcoin adapter.
    QueueFull,
}

impl fmt::Display for SendTransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedTransaction => {
                write!(f, "Can't deserialize transaction because it's malformed.")
            }
            Self::QueueFull => {
                write!(
                    f,
                    "Request can not be enqueued because the queue has reached its capacity. Please retry later."
                )
            }
        }
    }
}

/// A request to update the canister's config.
#[derive(CandidType, Deserialize, Default, Serialize)]
pub struct SetConfigRequest {
    pub stability_threshold: Option<u128>,

    /// Whether or not to enable/disable syncing of blocks from the network.
    pub syncing: Option<Flag>,

    /// The fees to charge for the various endpoints.
    pub fees: Option<Fees>,

    /// Whether or not to enable/disable the bitcoin apis.
    pub api_access: Option<Flag>,

    /// Whether or not to enable/disable the bitcoin apis if not fully synced.
    pub disable_api_if_not_fully_synced: Option<Flag>,

    /// The principal of the watchdog canister.
    /// The watchdog canister has the authority to disable the Bitcoin canister's API
    /// if it suspects that there is a problem.
    pub watchdog_canister: Option<Option<Principal>>,

    /// If enabled, fee percentiles are only computed when requested.
    /// Otherwise, they are computed whenever we receive a new block.
    pub lazily_evaluate_fee_percentiles: Option<Flag>,

    /// If enabled, continuously burns all cycles in the canister's balance
    /// to count towards the IC's burn rate.
    pub burn_cycles: Option<Flag>,
}

#[derive(CandidType, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Debug, Default)]
pub enum Flag {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
}

/// The config used to initialize the canister.
///
/// This struct is equivalent to `Config`, except that all its fields are optional.
/// Fields that are not specified here are loaded with their default value. See
/// `Config::default()`.
#[derive(CandidType, Deserialize, Debug, Default)]
pub struct InitConfig {
    pub stability_threshold: Option<u128>,
    pub network: Option<Network>,
    pub blocks_source: Option<Principal>,
    pub syncing: Option<Flag>,
    pub fees: Option<Fees>,
    pub api_access: Option<Flag>,
    pub disable_api_if_not_fully_synced: Option<Flag>,
    pub watchdog_canister: Option<Option<Principal>>,
    pub burn_cycles: Option<Flag>,
    pub lazily_evaluate_fee_percentiles: Option<Flag>,
}

/// The config of the canister.
#[derive(CandidType, Deserialize, Debug)]
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

    /// Flag to control access to the apis provided by the canister.
    pub api_access: Flag,

    /// Flag to determine if the API should be automatically disabled if
    /// the canister isn't fully synced.
    pub disable_api_if_not_fully_synced: Flag,

    /// The principal of the watchdog canister.
    /// The watchdog canister has the authority to disable the Bitcoin canister's API
    /// if it suspects that there is a problem.
    pub watchdog_canister: Option<Principal>,

    /// If enabled, continuously burns all cycles in its balance
    /// (to count towards the IC's burn rate).
    pub burn_cycles: Flag,

    /// If enabled, fee percentiles are only computed when requested.
    /// Otherwise, they are computed whenever we receive a new block.
    pub lazily_evaluate_fee_percentiles: Flag,
}

impl From<InitConfig> for Config {
    fn from(init_config: InitConfig) -> Self {
        let mut config = Config::default();

        if let Some(stability_threshold) = init_config.stability_threshold {
            config.stability_threshold = stability_threshold;
        }

        if let Some(network) = init_config.network {
            config.network = network;
        }

        if let Some(blocks_source) = init_config.blocks_source {
            config.blocks_source = blocks_source;
        }

        if let Some(syncing) = init_config.syncing {
            config.syncing = syncing;
        }

        let fees_explicitly_set = init_config.fees.is_some();
        if let Some(fees) = init_config.fees {
            config.fees = fees;
        }

        if let Some(api_access) = init_config.api_access {
            config.api_access = api_access;
        }

        if let Some(disable_api_if_not_fully_synced) = init_config.disable_api_if_not_fully_synced {
            config.disable_api_if_not_fully_synced = disable_api_if_not_fully_synced;
        }

        if let Some(watchdog_canister) = init_config.watchdog_canister {
            config.watchdog_canister = watchdog_canister;
        }

        if let Some(burn_cycles) = init_config.burn_cycles {
            config.burn_cycles = burn_cycles;
        }

        if let Some(lazily_evaluate_fee_percentiles) = init_config.lazily_evaluate_fee_percentiles {
            config.lazily_evaluate_fee_percentiles = lazily_evaluate_fee_percentiles;
        }

        // Config post-processing.
        if !fees_explicitly_set {
            config.fees = match config.network {
                Network::Mainnet => Fees::mainnet(),
                Network::Testnet => Fees::testnet(),
                Network::Regtest => config.fees, // Keep unchanged for regtest.
            };
        }

        config
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            stability_threshold: DEFAULT_STABILITY_THRESHOLD,
            network: Network::Regtest,
            blocks_source: Principal::management_canister(),
            syncing: Flag::Enabled,
            fees: Fees::default(),
            api_access: Flag::Enabled,
            disable_api_if_not_fully_synced: Flag::Enabled,
            watchdog_canister: None,
            burn_cycles: Flag::Disabled,
            lazily_evaluate_fee_percentiles: Flag::Disabled,
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

    #[serde(default)]
    /// The base fee to charge for all `get_block_headers` requests.
    pub get_block_headers_base: u128,

    #[serde(default)]
    /// The number of cycles to charge per 10 instructions.
    pub get_block_headers_cycles_per_ten_instructions: u128,

    #[serde(default)]
    /// The maximum amount of cycles that can be charged in a `get_block_headers` request.
    /// A request must send at least this amount for it to be accepted.
    pub get_block_headers_maximum: u128,
}

impl Fees {
    pub fn testnet() -> Self {
        // https://internetcomputer.org/docs/references/bitcoin-how-it-works#bitcoin-testnet
        Self {
            get_utxos_base: 20_000_000,
            get_utxos_cycles_per_ten_instructions: 4,
            get_utxos_maximum: 4_000_000_000,

            get_current_fee_percentiles: 4_000_000,
            get_current_fee_percentiles_maximum: 40_000_000,

            get_balance: 4_000_000,
            get_balance_maximum: 40_000_000,

            send_transaction_base: 2_000_000_000,
            send_transaction_per_byte: 8_000_000,

            get_block_headers_base: 20_000_000,
            get_block_headers_cycles_per_ten_instructions: 4,
            get_block_headers_maximum: 4_000_000_000,
        }
    }

    pub fn mainnet() -> Self {
        // https://internetcomputer.org/docs/references/bitcoin-how-it-works#bitcoin-mainnet
        Self {
            get_utxos_base: 50_000_000,
            get_utxos_cycles_per_ten_instructions: 10,
            get_utxos_maximum: 10_000_000_000,

            get_current_fee_percentiles: 10_000_000,
            get_current_fee_percentiles_maximum: 100_000_000,

            get_balance: 10_000_000,
            get_balance_maximum: 100_000_000,

            send_transaction_base: 5_000_000_000,
            send_transaction_per_byte: 20_000_000,

            get_block_headers_base: 50_000_000,
            get_block_headers_cycles_per_ten_instructions: 10,
            get_block_headers_maximum: 10_000_000_000,
        }
    }
}

/// Information about the blockchain as seen by the canister.
///
/// Currently returns information about the main chain tip. The main chain is the
/// canister's best guess at what the Bitcoin network considers the canonical chain.
/// It is defined as the longest chain with an "uncontested" tip — meaning there
/// exists no other block at the same height as the tip.
#[derive(CandidType, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct BlockchainInfo {
    /// The height of the main chain tip.
    pub height: Height,
    /// The hash of the tip block.
    pub block_hash: BlockHash,
    /// Unix timestamp of the tip block (seconds since epoch).
    pub timestamp: u32,
    /// Difficulty of the tip block.
    pub difficulty: u128,
    /// Total number of UTXOs up to the main chain tip (stable + unstable main chain blocks).
    pub utxos_length: u64,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_debug_formatter_is_enabled() {
        // Verify that debug formatter for Config is enabled.
        // This might be important for logging and debugging purposes.
        assert!(
            !format!("{:?}", Config::default()).is_empty(),
            "Config should be printable using debug formatter {{:?}}."
        );
    }

    #[test]
    fn can_extract_bytes_from_txid() {
        let tx_id = Txid([1; 32]);
        let tx: [u8; 32] = tx_id.into();
        assert_eq!(tx, [1; 32]);
    }
}
