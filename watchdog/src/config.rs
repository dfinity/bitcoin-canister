use crate::bitcoin_block_apis::{BitcoinMainnetExplorerBlockApi, DogecoinMainnetExplorerBlockApi};
use crate::bitcoin_block_apis::{BitcoinTestnetExplorerBlockApi, BlockApi};
use candid::CandidType;
use candid::Principal;
use serde::{Deserialize, Serialize};

/// Mainnet bitcoin canister principal.
const MAINNET_BITCOIN_CANISTER_PRINCIPAL: &str = "ghsi2-tqaaa-aaaan-aaaca-cai";

/// Testnet bitcoin canister principal.
const TESTNET_BITCOIN_CANISTER_PRINCIPAL: &str = "g4xu7-jiaaa-aaaan-aaaaq-cai";

/// Mainnet dogecoin canister principal.
const MAINNET_DOGECOIN_CANISTER_PRINCIPAL: &str = "gordg-fyaaa-aaaan-aaadq-cai";

/// Mainnet dogecoin staging canister principal.
const MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL: &str = "bhuiy-ciaaa-aaaad-abwea-cai";

/// The number of seconds to wait before the first data fetch.
const DELAY_BEFORE_FIRST_FETCH_SEC: u64 = 1;

/// The number of seconds to wait between all the other data fetches.
const BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC: u64 = 300;

/// The number of seconds to wait between all the other data fetches for the Dogecoin network.
const DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC: u64 = 30;

/// Canister to monitor.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub enum Canister {
    #[serde(rename = "bitcoin_mainnet")]
    BitcoinMainnet,

    #[serde(rename = "bitcoin_testnet")]
    BitcoinTestnet,

    #[serde(rename = "dogecoin_mainnet")]
    DogecoinMainnet,

    #[serde(rename = "dogecoin_mainnet_staging")]
    DogecoinMainnetStaging,
}

#[derive(Copy, Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    BitcoinMainnet,
    BitcoinTestnet,
    DogecoinMainnet,
}

/// Watchdog canister configuration.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// The network to use.
    pub network: Network,

    /// Below this threshold, the canister is considered to be behind.
    blocks_behind_threshold: u64,

    /// Above this threshold, the canister is considered to be ahead.
    blocks_ahead_threshold: u64,

    /// The minimum number of explorers to compare against.
    pub min_explorers: u64,

    /// Monitored canister principal.
    pub canister_principal: Principal,

    /// The number of seconds to wait before the first data fetch.
    pub delay_before_first_fetch_sec: u64,

    /// The number of seconds to wait between all the other data fetches.
    pub interval_between_fetches_sec: u64,

    /// Explorers to use for fetching block data.
    pub explorers: Vec<BlockApi>,
}

impl Config {
    /// Creates a new configuration for the Bitcoin mainnet.
    pub fn bitcoin_mainnet() -> Self {
        Self {
            network: Network::BitcoinMainnet,
            blocks_behind_threshold: 2,
            blocks_ahead_threshold: 2,
            min_explorers: 3,
            canister_principal: Principal::from_text(MAINNET_BITCOIN_CANISTER_PRINCIPAL).unwrap(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![
                BitcoinMainnetExplorerBlockApi::ApiBitapsCom.into(),
                BitcoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                BitcoinMainnetExplorerBlockApi::BitcoinExplorerOrg.into(),
                BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                BitcoinMainnetExplorerBlockApi::ChainApiBtcCom.into(),
                BitcoinMainnetExplorerBlockApi::Mempool.into(),
            ],
        }
    }

    /// Creates a new configuration for the Bitcoin testnet.
    pub fn bitcoin_testnet() -> Self {
        Self {
            network: Network::BitcoinTestnet,
            blocks_behind_threshold: 1000,
            blocks_ahead_threshold: 1000,
            min_explorers: 1,
            canister_principal: Principal::from_text(TESTNET_BITCOIN_CANISTER_PRINCIPAL).unwrap(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![BitcoinTestnetExplorerBlockApi::Mempool.into()],
        }
    }

    /// Creates a new configuration for the Dogecoin mainnet.
    pub fn dogecoin_mainnet(staging_canister: bool) -> Self {
        Self {
            network: Network::DogecoinMainnet,
            blocks_behind_threshold: 2,
            blocks_ahead_threshold: 2,
            min_explorers: 2,
            canister_principal: if staging_canister {
                Principal::from_text(MAINNET_DOGECOIN_CANISTER_PRINCIPAL).unwrap()
            } else {
                Principal::from_text(MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL).unwrap()
            },
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![
                DogecoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                DogecoinMainnetExplorerBlockApi::TokenView.into(),
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

    /// Returns the canister metrics endpoint.
    pub fn get_canister_endpoint(&self) -> String {
        let principal = self.canister_principal.to_text();
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

    /// Mainnet dogecoin canister endpoint.
    const MAINNET_DOGECOIN_CANISTER_ENDPOINT: &str =
        "https://gordg-fyaaa-aaaan-aaadq-cai.raw.ic0.app/metrics";

    /// Mainnet dogecoin staging canister endpoint.
    const MAINNET_DOGECOIN_STAGING_CANISTER_ENDPOINT: &str =
        "https://bhuiy-ciaaa-aaaad-abwea-cai.raw.ic0.app/metrics";

    #[test]
    fn test_bitcoin_canister_endpoint_contains_principal_mainnet() {
        assert!(MAINNET_BITCOIN_CANISTER_ENDPOINT.contains(MAINNET_BITCOIN_CANISTER_PRINCIPAL));
    }

    #[test]
    fn test_bitcoin_canister_endpoint_contains_principal_testnet() {
        assert!(TESTNET_BITCOIN_CANISTER_ENDPOINT.contains(TESTNET_BITCOIN_CANISTER_PRINCIPAL));
    }

    #[test]
    fn test_dogecoin_canister_endpoint_contains_principal_mainnet() {
        assert!(MAINNET_DOGECOIN_CANISTER_ENDPOINT.contains(MAINNET_DOGECOIN_CANISTER_PRINCIPAL));
    }

    #[test]
    fn test_dogecoin_canister_endpoint_contains_principal_mainnet_staging() {
        assert!(MAINNET_DOGECOIN_STAGING_CANISTER_ENDPOINT
            .contains(MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL));
    }

    #[test]
    fn test_config_mainnet() {
        let config = Config::bitcoin_mainnet();
        assert_eq!(config.network, Network::BitcoinMainnet);
        assert_eq!(
            config.canister_principal,
            Principal::from_text(MAINNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            config.get_canister_endpoint(),
            MAINNET_BITCOIN_CANISTER_ENDPOINT
        );
    }

    #[test]
    fn test_config_testnet() {
        let config = Config::bitcoin_testnet();
        assert_eq!(config.network, Network::BitcoinTestnet);
        assert_eq!(
            config.canister_principal,
            Principal::from_text(TESTNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            config.get_canister_endpoint(),
            TESTNET_BITCOIN_CANISTER_ENDPOINT
        );
    }

    #[test]
    fn test_config_dogecoin_mainnet() {
        let staging_canister = false;
        let config = Config::dogecoin_mainnet(staging_canister);
        assert_eq!(config.network, Network::DogecoinMainnet);
        assert_eq!(
            config.canister_principal,
            Principal::from_text(MAINNET_DOGECOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            config.get_canister_endpoint(),
            MAINNET_DOGECOIN_CANISTER_ENDPOINT
        );
    }

    #[test]
    fn test_config_dogecoin_mainnet_staging() {
        let staging_canister = true;
        let config = Config::dogecoin_mainnet(staging_canister);
        assert_eq!(config.network, Network::DogecoinMainnet);
        assert_eq!(
            config.canister_principal,
            Principal::from_text(MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            config.get_canister_endpoint(),
            MAINNET_DOGECOIN_STAGING_CANISTER_ENDPOINT
        );
    }
}
