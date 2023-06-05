use crate::bitcoin_block_apis::BitcoinBlockApi;
use candid::CandidType;
use ic_cdk::export::Principal;
use serde::{Deserialize, Serialize};

/// The Bitcoin network to use.
const BITCOIN_NETWORK: BitcoinNetwork = BitcoinNetwork::Mainnet;

/// Below this threshold, the canister is considered to be behind.
/// This value is positive, but it will be converted to negative.
const BLOCKS_BEHIND_THRESHOLD: u64 = 2;

/// Above this threshold, the canister is considered to be ahead.
const BLOCKS_AHEAD_THRESHOLD: u64 = 2;

/// The minimum number of explorers to compare against.
const MIN_EXPLORERS: u64 = 2;

/// Mainnet bitcoin canister principal.
const MAINNET_BITCOIN_CANISTER_PRINCIPAL: &str = "ghsi2-tqaaa-aaaan-aaaca-cai";

/// Testnet bitcoin canister principal.
const TESTNET_BITCOIN_CANISTER_PRINCIPAL: &str = "g4xu7-jiaaa-aaaan-aaaaq-cai";

/// The number of seconds to wait before the first data fetch.
const DELAY_BEFORE_FIRST_FETCH_SEC: u64 = 1;

/// The number of seconds to wait between all the other data fetches.
const INTERVAL_BETWEEN_FETCHES_SEC: u64 = 300;

/// Bitcoin network.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitcoinNetwork {
    #[serde(rename = "mainnet")]
    Mainnet,

    #[serde(rename = "testnet")]
    Testnet,
}

/// Watchdog canister configuration.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// The Bitcoin network to use.
    pub bitcoin_network: BitcoinNetwork,

    /// Below this threshold, the canister is considered to be behind.
    blocks_behind_threshold: u64,

    /// Above this threshold, the canister is considered to be ahead.
    blocks_ahead_threshold: u64,

    /// The minimum number of explorers to compare against.
    pub min_explorers: u64,

    /// Bitcoin canister principal.
    pub bitcoin_canister_principal: Principal,

    /// The number of seconds to wait before the first data fetch.
    pub delay_before_first_fetch_sec: u64,

    /// The number of seconds to wait between all the other data fetches.
    pub interval_between_fetches_sec: u64,

    /// Bitcoin Explorers to use for fetching bitcoin block data.
    pub explorers: Vec<BitcoinBlockApi>,
}

impl Config {
    /// Creates a new configuration depending on the Bitcoin network.
    pub fn new() -> Self {
        match BITCOIN_NETWORK {
            BitcoinNetwork::Mainnet => Self::mainnet(),
            BitcoinNetwork::Testnet => Self::testnet(),
        }
    }

    /// Creates a new configuration for the mainnet.
    pub fn mainnet() -> Self {
        Self {
            bitcoin_network: BitcoinNetwork::Mainnet,
            blocks_behind_threshold: BLOCKS_BEHIND_THRESHOLD,
            blocks_ahead_threshold: BLOCKS_AHEAD_THRESHOLD,
            min_explorers: MIN_EXPLORERS,
            bitcoin_canister_principal: Principal::from_text(MAINNET_BITCOIN_CANISTER_PRINCIPAL)
                .unwrap(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![
                BitcoinBlockApi::ApiBitapsComMainnet,
                BitcoinBlockApi::ApiBlockchairComMainnet,
                BitcoinBlockApi::ApiBlockcypherComMainnet,
                BitcoinBlockApi::BlockchainInfoMainnet,
                BitcoinBlockApi::BlockstreamInfoMainnet,
                BitcoinBlockApi::ChainApiBtcComMainnet,
            ],
        }
    }

    /// Creates a new configuration for the testnet.
    pub fn testnet() -> Self {
        Self {
            bitcoin_network: BitcoinNetwork::Testnet,
            blocks_behind_threshold: BLOCKS_BEHIND_THRESHOLD,
            blocks_ahead_threshold: BLOCKS_AHEAD_THRESHOLD,
            min_explorers: MIN_EXPLORERS,
            bitcoin_canister_principal: Principal::from_text(TESTNET_BITCOIN_CANISTER_PRINCIPAL)
                .unwrap(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![
                BitcoinBlockApi::ApiBitapsComTestnet,
                BitcoinBlockApi::ApiBlockchairComTestnet,
                BitcoinBlockApi::ApiBlockcypherComTestnet,
                BitcoinBlockApi::BlockstreamInfoTestnet,
            ],
        }
    }

    /// Returns the number of blocks behind threshold as a negative number.
    pub fn get_blocks_behind_threshold(&self) -> i64 {
        -(self.blocks_behind_threshold as i64)
    }

    /// Returns the number of blocks ahead threshold as a positive number.
    pub fn get_blocks_ahead_threshold(&self) -> i64 {
        self.blocks_ahead_threshold as i64
    }

    /// Returns the Bitcoin canister metrics endpoint.
    pub fn get_bitcoin_canister_endpoint(&self) -> String {
        let principal = self.bitcoin_canister_principal.to_text();
        format!("https://{principal}.raw.ic0.app/metrics")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Mainnet bitcoin canister endpoint.
    const MAINNET_BITCOIN_CANISTER_ENDPOINT: &str =
        "https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics";

    /// Testnet bitcoin canister endpoint.
    const TESTNET_BITCOIN_CANISTER_ENDPOINT: &str =
        "https://g4xu7-jiaaa-aaaan-aaaaq-cai.raw.ic0.app/metrics";

    #[test]
    fn test_bitcoin_canister_endpoint_contains_principal_mainnet() {
        assert!(MAINNET_BITCOIN_CANISTER_ENDPOINT.contains(MAINNET_BITCOIN_CANISTER_PRINCIPAL));
    }

    #[test]
    fn test_bitcoin_canister_endpoint_contains_principal_testnet() {
        assert!(TESTNET_BITCOIN_CANISTER_ENDPOINT.contains(TESTNET_BITCOIN_CANISTER_PRINCIPAL));
    }

    #[test]
    fn test_config_mainnet() {
        let config = Config::mainnet();
        assert_eq!(config.bitcoin_network, BitcoinNetwork::Mainnet);
        assert_eq!(
            config.bitcoin_canister_principal,
            Principal::from_text(MAINNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            config.get_bitcoin_canister_endpoint(),
            MAINNET_BITCOIN_CANISTER_ENDPOINT
        );
    }

    #[test]
    fn test_config_testnet() {
        let config = Config::testnet();
        assert_eq!(config.bitcoin_network, BitcoinNetwork::Testnet);
        assert_eq!(
            config.bitcoin_canister_principal,
            Principal::from_text(TESTNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            config.get_bitcoin_canister_endpoint(),
            TESTNET_BITCOIN_CANISTER_ENDPOINT
        );
    }
}
