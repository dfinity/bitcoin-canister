use candid::CandidType;
use serde::{Deserialize, Serialize};

/// The Bitcoin network to use.
const BITCOIN_NETWORK: BitcoinNetwork = BitcoinNetwork::Mainnet;

/// Below this threshold, the canister is considered to be behind.
const BLOCKS_BEHIND_THRESHOLD: u64 = 2;

/// Above this threshold, the canister is considered to be ahead.
const BLOCKS_AHEAD_THRESHOLD: u64 = 2;

/// The minimum number of explorers to compare against.
const MIN_EXPLORERS: u64 = 3;

/// Bitcoin canister endpoint.
const MAINNET_BITCOIN_CANISTER_ENDPOINT: &str =
    "https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics";

const TESTNET_BITCOIN_CANISTER_ENDPOINT: &str =
    "https://g4xu7-jiaaa-aaaan-aaaaq-cai.raw.ic0.app/metrics";

/// The number of seconds to wait before the first data fetch.
const DELAY_BEFORE_FIRST_FETCH_SEC: u64 = 1;

/// The number of seconds to wait between all the other data fetches.
const INTERVAL_BETWEEN_FETCHES_SEC: u64 = 120;

#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitcoinNetwork {
    Mainnet,
    Testnet,
}

#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// The Bitcoin network to use.
    pub bitcoin_network: BitcoinNetwork,

    /// Below this threshold, the canister is considered to be behind.
    pub blocks_behind_threshold: u64,

    /// Above this threshold, the canister is considered to be ahead.
    pub blocks_ahead_threshold: u64,

    /// The minimum number of explorers to compare against.
    pub min_explorers: u64,

    /// Bitcoin canister endpoint.
    pub bitcoin_canister_endpoint: String,

    /// The number of seconds to wait before the first data fetch.
    pub delay_before_first_fetch_sec: u64,

    /// The number of seconds to wait between all the other data fetches.
    pub interval_between_fetches_sec: u64,
}

impl Config {
    pub fn new() -> Self {
        match BITCOIN_NETWORK {
            BitcoinNetwork::Mainnet => Self::mainnet(),
            BitcoinNetwork::Testnet => Self::testnet(),
        }
    }

    pub fn mainnet() -> Self {
        Self {
            bitcoin_network: BitcoinNetwork::Mainnet,
            blocks_behind_threshold: BLOCKS_BEHIND_THRESHOLD,
            blocks_ahead_threshold: BLOCKS_AHEAD_THRESHOLD,
            min_explorers: MIN_EXPLORERS,
            bitcoin_canister_endpoint: MAINNET_BITCOIN_CANISTER_ENDPOINT.to_string(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: INTERVAL_BETWEEN_FETCHES_SEC,
        }
    }

    pub fn testnet() -> Self {
        Self {
            bitcoin_network: BitcoinNetwork::Testnet,
            blocks_behind_threshold: BLOCKS_BEHIND_THRESHOLD,
            blocks_ahead_threshold: BLOCKS_AHEAD_THRESHOLD,
            min_explorers: MIN_EXPLORERS,
            bitcoin_canister_endpoint: TESTNET_BITCOIN_CANISTER_ENDPOINT.to_string(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: INTERVAL_BETWEEN_FETCHES_SEC,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
