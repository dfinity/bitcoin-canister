//! Types used in the interface of the Bitcoin Canister.

use candid::{CandidType, Deserialize, Principal};
use serde::Serialize;
use serde_bytes::ByteBuf;
use std::str::FromStr;

pub type Address = String;
pub type Satoshi = u64;
pub type MillisatoshiPerByte = u64;
pub type BlockHash = Vec<u8>;
pub type Height = u32;
pub type Page = ByteBuf;

#[derive(CandidType, Clone, Copy, Deserialize, Debug, Eq, PartialEq, Serialize, Hash)]
pub enum Network {
    #[serde(rename = "mainnet")]
    Mainnet,
    #[serde(rename = "testnet")]
    Testnet,
    #[serde(rename = "regtest")]
    Regtest,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
    Mainnet,
    #[allow(non_camel_case_types)]
    mainnet,
    Testnet,
    #[allow(non_camel_case_types)]
    testnet,
    Regtest,
    #[allow(non_camel_case_types)]
    regtest,
}

impl std::fmt::Display for NetworkInRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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

/// A reference to a transaction output.
#[derive(
    CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct OutPoint {
    /// A cryptographic hash of the transaction.
    /// A transaction can output multiple UTXOs.
    #[serde(with = "serde_bytes")]
    pub txid: Vec<u8>,
    /// The index of the output within the transaction.
    pub vout: u32,
}

/// An unspent transaction output.
#[derive(CandidType, Debug, Deserialize, PartialEq, Serialize, Clone, Hash, Eq)]
pub struct Utxo {
    pub outpoint: OutPoint,
    pub value: Satoshi,
    pub height: u32,
}

impl std::cmp::PartialOrd for Utxo {
    fn partial_cmp(&self, other: &Utxo) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Utxo {
    fn cmp(&self, other: &Utxo) -> std::cmp::Ordering {
        // The output point uniquely identifies an UTXO; there is no point in
        // comparing the other fields.
        self.outpoint.cmp(&other.outpoint)
    }
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
    pub tip_height: u32,
    pub next_page: Option<Page>,
}

/// Errors when processing a `get_utxos` request.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq, Clone)]
pub enum GetUtxosError {
    MalformedAddress,
    MinConfirmationsTooLarge { given: u32, max: u32 },
    UnknownTipBlockHash { tip_block_hash: BlockHash },
    MalformedPage { err: String },
}

/// A request for getting the current fee percentiles.
#[derive(CandidType, Debug, Deserialize, PartialEq, Eq)]
pub struct GetCurrentFeePercentilesRequest {
    pub network: NetworkInRequest,
}

impl std::fmt::Display for GetUtxosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    MinConfirmationsTooLarge { given: u32, max: u32 },
}

impl std::fmt::Display for GetBalanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl std::fmt::Display for SendTransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
}

#[derive(CandidType, Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Debug, Default)]
pub enum Flag {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
}

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

    /// Flag to control access to the apis provided by the canister.
    pub api_access: Flag,

    /// Flag to determine if the API should be automatically disabled if
    /// the canister isn't fully synced.
    pub disable_api_if_not_fully_synced: Flag,

    /// The principal of the watchdog canister.
    /// The watchdog canister has the authority to disable the Bitcoin canister's API
    /// if it suspects that there is a problem.
    pub watchdog_canister: Option<Principal>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            stability_threshold: 0,
            network: Network::Regtest,
            blocks_source: Principal::management_canister(),
            syncing: Flag::Enabled,
            fees: Fees::default(),
            api_access: Flag::Enabled,
            disable_api_if_not_fully_synced: Flag::Enabled,
            watchdog_canister: None,
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
