use crate::bitcoin_block_apis::BitcoinBlockApi;
use candid::CandidType;
use candid::Principal;
use serde::{Deserialize, Serialize};

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
    /// Creates a new configuration for the mainnet.
    pub fn mainnet() -> Self {
        #[cfg(not(feature = "health_status_test"))]
        const MIN_EXPLORERS: u64 = 3;

        // Due to dfx not supporting ipv4, only two explorers support ipv6 and
        // can be part of the health_status_test.
        #[cfg(feature = "health_status_test")]
        const MIN_EXPLORERS: u64 = 2;

        Self {
            bitcoin_network: BitcoinNetwork::Mainnet,
            blocks_behind_threshold: 2,
            blocks_ahead_threshold: 2,
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
            blocks_behind_threshold: 100,
            blocks_ahead_threshold: 100,
            min_explorers: 2,
            bitcoin_canister_principal: Principal::from_text(TESTNET_BITCOIN_CANISTER_PRINCIPAL)
                .unwrap(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![
                // NOTE: Disabled due to flakiness.
                // BitcoinBlockApi::ApiBitapsComTestnet,
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
