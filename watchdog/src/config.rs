use crate::block_apis::BitcoinTestnetExplorerBlockApi;
use crate::block_apis::{
    BitcoinMainnetExplorerBlockApi, CandidBlockApi, DogecoinMainnetExplorerBlockApi,
};
use candid::CandidType;
use candid::Principal;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Mainnet bitcoin canister principal.
const MAINNET_BITCOIN_CANISTER_PRINCIPAL: &str = "ghsi2-tqaaa-aaaan-aaaca-cai";

/// Testnet bitcoin canister principal.
const TESTNET_BITCOIN_CANISTER_PRINCIPAL: &str = "g4xu7-jiaaa-aaaan-aaaaq-cai";

/// Mainnet bitcoin staging canister principal.
const MAINNET_BITCOIN_STAGING_CANISTER_PRINCIPAL: &str = "axowo-ciaaa-aaaad-acs7q-cai";

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

    #[serde(rename = "bitcoin_mainnet_staging")]
    BitcoinMainnetStaging,

    #[serde(rename = "bitcoin_testnet")]
    BitcoinTestnet,

    #[serde(rename = "dogecoin_mainnet")]
    DogecoinMainnet,

    #[serde(rename = "dogecoin_mainnet_staging")]
    DogecoinMainnetStaging,
}

#[derive(Copy, Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    #[serde(rename = "bitcoin_mainnet")]
    BitcoinMainnet,
    #[serde(rename = "bitcoin_testnet")]
    BitcoinTestnet,
    #[serde(rename = "dogecoin_mainnet")]
    DogecoinMainnet,
}

/// Type of subnet on which the watchdog and target canisters are deployed.
#[derive(Copy, Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubnetType {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "application")]
    Application,
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
    pub explorers: Vec<CandidBlockApi>,

    /// Type of subnet on which the watchdog and target canisters are deployed.
    pub subnet_type: SubnetType,
}

impl Config {
    /// Creates a new configuration for the given canister.
    pub fn for_target(canister: Canister) -> Self {
        match canister {
            Canister::BitcoinMainnet => Self {
                network: Network::BitcoinMainnet,
                blocks_behind_threshold: 2,
                blocks_ahead_threshold: 2,
                min_explorers: 3,
                canister_principal: Principal::from_text(MAINNET_BITCOIN_CANISTER_PRINCIPAL)
                    .unwrap(),
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
                explorers: vec![
                    BitcoinMainnetExplorerBlockApi::ApiBitapsCom.into(),
                    BitcoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                    // BitcoinMainnetExplorerBlockApi::BlockexplorerOne.into(),
                    BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                    BitcoinMainnetExplorerBlockApi::Mempool.into(),
                ],
                subnet_type: SubnetType::System,
            },
            Canister::BitcoinMainnetStaging => Self {
                network: Network::BitcoinMainnet,
                blocks_behind_threshold: 2,
                blocks_ahead_threshold: 2,
                min_explorers: 3,
                canister_principal: Principal::from_text(
                    MAINNET_BITCOIN_STAGING_CANISTER_PRINCIPAL,
                )
                .unwrap(),
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
                explorers: vec![
                    BitcoinMainnetExplorerBlockApi::ApiBitapsCom.into(),
                    BitcoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                    // BitcoinMainnetExplorerBlockApi::BlockexplorerOne.into(),
                    BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                    BitcoinMainnetExplorerBlockApi::Mempool.into(),
                ],
                subnet_type: SubnetType::Application,
            },
            Canister::BitcoinTestnet => Self {
                network: Network::BitcoinTestnet,
                blocks_behind_threshold: 1000,
                blocks_ahead_threshold: 1000,
                min_explorers: 1,
                canister_principal: Principal::from_text(TESTNET_BITCOIN_CANISTER_PRINCIPAL)
                    .unwrap(),
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
                explorers: vec![BitcoinTestnetExplorerBlockApi::Mempool.into()],
                subnet_type: SubnetType::System,
            },
            Canister::DogecoinMainnet => Self {
                network: Network::DogecoinMainnet,
                blocks_behind_threshold: 2,
                blocks_ahead_threshold: 2,
                min_explorers: 2,
                canister_principal: Principal::from_text(MAINNET_DOGECOIN_CANISTER_PRINCIPAL)
                    .unwrap(),
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC,
                explorers: vec![
                    DogecoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    DogecoinMainnetExplorerBlockApi::TokenView.into(),
                ],
                subnet_type: SubnetType::System,
            },
            Canister::DogecoinMainnetStaging => Self {
                network: Network::DogecoinMainnet,
                blocks_behind_threshold: 2,
                blocks_ahead_threshold: 2,
                min_explorers: 2,
                canister_principal: Principal::from_text(
                    MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL,
                )
                .unwrap(),
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC,
                explorers: vec![
                    DogecoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    DogecoinMainnetExplorerBlockApi::TokenView.into(),
                ],
                subnet_type: SubnetType::Application,
            },
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
        let suffix = match self.subnet_type {
            SubnetType::System => "raw.ic0.app",
            SubnetType::Application => "raw.icp0.io",
        };
        format!("https://{principal}.{suffix}/metrics")
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::for_target(Canister::BitcoinMainnet)
    }
}

impl Storable for Config {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(encode(self))
    }

    fn into_bytes(self) -> Vec<u8> {
        encode(&self)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        decode(bytes.as_ref())
    }

    const BOUND: Bound = Bound::Unbounded;
}

fn encode(config: &Config) -> Vec<u8> {
    let mut buf = vec![];
    ciborium::ser::into_writer(config, &mut buf).expect("failed to encode state");
    buf
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
    ciborium::de::from_reader(bytes)
        .unwrap_or_else(|e| panic!("failed to decode state bytes {}: {e}", hex::encode(bytes)))
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::prelude::*;

    /// Mainnet bitcoin canister endpoint.
    const MAINNET_BITCOIN_CANISTER_ENDPOINT: &str =
        "https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics";

    /// Testnet bitcoin canister endpoint.
    const TESTNET_BITCOIN_CANISTER_ENDPOINT: &str =
        "https://g4xu7-jiaaa-aaaan-aaaaq-cai.raw.ic0.app/metrics";

    /// Mainnet bitcoin staging canister endpoint.
    const MAINNET_BITCOIN_STAGING_CANISTER_ENDPOINT: &str =
        "https://axowo-ciaaa-aaaad-acs7q-cai.raw.icp0.io/metrics";

    /// Mainnet dogecoin canister endpoint.
    const MAINNET_DOGECOIN_CANISTER_ENDPOINT: &str =
        "https://gordg-fyaaa-aaaan-aaadq-cai.raw.ic0.app/metrics";

    /// Mainnet dogecoin staging canister endpoint.
    const MAINNET_DOGECOIN_STAGING_CANISTER_ENDPOINT: &str =
        "https://bhuiy-ciaaa-aaaad-abwea-cai.raw.icp0.io/metrics";

    #[test]
    fn test_bitcoin_canister_endpoint_contains_principal_mainnet() {
        assert!(MAINNET_BITCOIN_CANISTER_ENDPOINT.contains(MAINNET_BITCOIN_CANISTER_PRINCIPAL));
    }

    #[test]
    fn test_bitcoin_canister_endpoint_contains_principal_testnet() {
        assert!(TESTNET_BITCOIN_CANISTER_ENDPOINT.contains(TESTNET_BITCOIN_CANISTER_PRINCIPAL));
    }

    #[test]
    fn test_bitcoin_canister_endpoint_contains_principal_mainnet_staging() {
        assert!(MAINNET_BITCOIN_STAGING_CANISTER_ENDPOINT
            .contains(MAINNET_BITCOIN_STAGING_CANISTER_PRINCIPAL));
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
        let config = Config::for_target(Canister::BitcoinMainnet);
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
    fn test_config_mainnet_staging() {
        let config = Config::for_target(Canister::BitcoinMainnetStaging);
        assert_eq!(config.network, Network::BitcoinMainnet);
        assert_eq!(
            config.canister_principal,
            Principal::from_text(MAINNET_BITCOIN_STAGING_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            config.get_canister_endpoint(),
            MAINNET_BITCOIN_STAGING_CANISTER_ENDPOINT
        );
    }

    #[test]
    fn test_config_testnet() {
        let config = Config::for_target(Canister::BitcoinTestnet);
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
        let config = Config::for_target(Canister::DogecoinMainnet);
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
        let config = Config::for_target(Canister::DogecoinMainnetStaging);
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

    proptest! {
        #[test]
        fn test_config_encode_decode(canister in prop_oneof![
            Just(Canister::BitcoinMainnet),
            Just(Canister::BitcoinMainnetStaging),
            Just(Canister::BitcoinTestnet),
            Just(Canister::DogecoinMainnet),
            Just(Canister::DogecoinMainnetStaging),
        ]) {
            let config = Config::for_target(canister);
            let encoded = encode(&config);
            let decoded: Config = decode(&encoded);
            assert_eq!(config, decoded);
        }
    }
}
