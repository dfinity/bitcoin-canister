use crate::block_apis::{
    BitcoinMainnetExplorerBlockApi, BitcoinMainnetProviderBlockApi, BitcoinTestnetExplorerBlockApi,
    BitcoinTestnetProviderBlockApi, BlockApi, DogecoinMainnetExplorerBlockApi,
    DogecoinProviderBlockApi,
};
use candid::CandidType;
use candid::Principal;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Debug;

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
const DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC: u64 = 60;

/// Canister to monitor.
#[derive(Copy, Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
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

impl Default for Canister {
    fn default() -> Self {
        Canister::BitcoinMainnet
    }
}

impl Storable for Canister {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut buf = vec![];
        ciborium::ser::into_writer(self, &mut buf).expect("failed to encode canister");
        Cow::Owned(buf)
    }

    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).expect("failed to decode canister")
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 64,
        is_fixed_size: false,
    };
}

impl Canister {
    pub fn network(&self) -> Network {
        match self {
            Canister::BitcoinMainnet | Canister::BitcoinMainnetStaging => Network::BitcoinMainnet,
            Canister::BitcoinTestnet => Network::BitcoinTestnet,
            Canister::DogecoinMainnet | Canister::DogecoinMainnetStaging => Network::DogecoinMainnet,
        }
    }

    pub fn canister_principal(&self) -> Principal {
        Principal::from_text(match self {
            Canister::BitcoinMainnet => MAINNET_BITCOIN_CANISTER_PRINCIPAL,
            Canister::BitcoinMainnetStaging => MAINNET_BITCOIN_STAGING_CANISTER_PRINCIPAL,
            Canister::BitcoinTestnet => TESTNET_BITCOIN_CANISTER_PRINCIPAL,
            Canister::DogecoinMainnet => MAINNET_DOGECOIN_CANISTER_PRINCIPAL,
            Canister::DogecoinMainnetStaging => MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL,
        })
        .unwrap()
    }

    pub fn subnet_type(&self) -> SubnetType {
        match self {
            Canister::BitcoinMainnet | Canister::BitcoinTestnet | Canister::DogecoinMainnet => {
                SubnetType::System
            }
            Canister::BitcoinMainnetStaging | Canister::DogecoinMainnetStaging => {
                SubnetType::Application
            }
        }
    }

    pub fn canister_api(&self) -> BlockApi {
        match self {
            Canister::BitcoinMainnet | Canister::BitcoinMainnetStaging => {
                BlockApi::BitcoinMainnetProvider(BitcoinMainnetProviderBlockApi::BitcoinCanister)
            }
            Canister::BitcoinTestnet => {
                BlockApi::BitcoinTestnetProvider(BitcoinTestnetProviderBlockApi::BitcoinCanister)
            }
            Canister::DogecoinMainnet | Canister::DogecoinMainnetStaging => {
                BlockApi::DogecoinProvider(DogecoinProviderBlockApi::DogecoinCanister)
            }
        }
    }

    pub fn explorers(&self) -> Vec<BlockApi> {
        match self {
            Canister::BitcoinMainnet | Canister::BitcoinMainnetStaging => vec![
                BlockApi::BitcoinMainnetProvider(BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBitapsCom,
                )),
                BlockApi::BitcoinMainnetProvider(BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBlockchairCom,
                )),
                BlockApi::BitcoinMainnetProvider(BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom,
                )),
                BlockApi::BitcoinMainnetProvider(BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockchainInfo,
                )),
                BlockApi::BitcoinMainnetProvider(BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockstreamInfo,
                )),
                BlockApi::BitcoinMainnetProvider(BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::Mempool,
                )),
            ],
            Canister::BitcoinTestnet => vec![BlockApi::BitcoinTestnetProvider(
                BitcoinTestnetProviderBlockApi::Testnet(BitcoinTestnetExplorerBlockApi::Mempool),
            )],
            Canister::DogecoinMainnet | Canister::DogecoinMainnetStaging => vec![
                BlockApi::DogecoinProvider(DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::ApiBlockchairCom,
                )),
                BlockApi::DogecoinProvider(DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom,
                )),
                BlockApi::DogecoinProvider(DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::TokenView,
                )),
            ],
        }
    }

    pub fn all_providers(&self) -> Vec<BlockApi> {
        let mut providers = self.explorers();
        providers.push(self.canister_api());
        providers
    }

    pub fn get_canister_endpoint(&self) -> String {
        let principal = self.canister_principal().to_text();
        let suffix = match self.subnet_type() {
            SubnetType::System => "raw.ic0.app",
            SubnetType::Application => "raw.icp0.io",
        };
        format!("https://{principal}.{suffix}/metrics")
    }
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

/// Stored configuration for runtime-modifiable values.
/// Static values (principal, explorers, subnet_type) come from `Canister`.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredConfig {
    /// Below this threshold, the canister is considered to be behind.
    pub blocks_behind_threshold: u64,

    /// Above this threshold, the canister is considered to be ahead.
    pub blocks_ahead_threshold: u64,

    /// The minimum number of explorers to compare against.
    pub min_explorers: u64,

    /// The number of seconds to wait before the first data fetch.
    pub delay_before_first_fetch_sec: u64,

    /// The number of seconds to wait between all the other data fetches.
    pub interval_between_fetches_sec: u64,
}

impl StoredConfig {
    /// Creates a new configuration with defaults for the given canister.
    pub fn for_target(canister: Canister) -> Self {
        match canister {
            Canister::BitcoinMainnet | Canister::BitcoinMainnetStaging => Self {
                blocks_behind_threshold: 2,
                blocks_ahead_threshold: 2,
                min_explorers: 3,
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            },
            Canister::BitcoinTestnet => Self {
                blocks_behind_threshold: 1000,
                blocks_ahead_threshold: 1000,
                min_explorers: 1,
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            },
            Canister::DogecoinMainnet | Canister::DogecoinMainnetStaging => Self {
                blocks_behind_threshold: 4,
                blocks_ahead_threshold: 4,
                min_explorers: 2,
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC,
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
}

impl Default for StoredConfig {
    fn default() -> Self {
        StoredConfig::for_target(Canister::default())
    }
}

/// Combined configuration for the candid API response.
/// This combines static values from `Canister` with modifiable values from `StoredConfig`.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidConfig {
    /// The network to use.
    pub network: Network,

    /// Below this threshold, the canister is considered to be behind.
    pub blocks_behind_threshold: u64,

    /// Above this threshold, the canister is considered to be ahead.
    pub blocks_ahead_threshold: u64,

    /// The minimum number of explorers to compare against.
    pub min_explorers: u64,

    /// Monitored canister principal.
    pub canister_principal: Principal,

    /// The number of seconds to wait before the first data fetch.
    pub delay_before_first_fetch_sec: u64,

    /// The number of seconds to wait between all the other data fetches.
    pub interval_between_fetches_sec: u64,

    /// Explorers to use for fetching block data.
    pub explorers: Vec<String>,

    /// Type of subnet on which the watchdog and target canisters are deployed.
    pub subnet_type: SubnetType,
}

impl CandidConfig {
    /// Combines static values from `Canister` with modifiable values from `StoredConfig`.
    pub fn from_parts(canister: Canister, config: StoredConfig) -> Self {
        Self {
            network: canister.network(),
            blocks_behind_threshold: config.blocks_behind_threshold,
            blocks_ahead_threshold: config.blocks_ahead_threshold,
            min_explorers: config.min_explorers,
            canister_principal: canister.canister_principal(),
            delay_before_first_fetch_sec: config.delay_before_first_fetch_sec,
            interval_between_fetches_sec: config.interval_between_fetches_sec,
            explorers: canister.explorers().iter().map(|e| e.to_string()).collect(),
            subnet_type: canister.subnet_type(),
        }
    }
}

impl Storable for StoredConfig {
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

fn encode(config: &StoredConfig) -> Vec<u8> {
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
        let canister = Canister::BitcoinMainnet;
        assert_eq!(canister.network(), Network::BitcoinMainnet);
        assert_eq!(
            canister.canister_principal(),
            Principal::from_text(MAINNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            canister.get_canister_endpoint(),
            MAINNET_BITCOIN_CANISTER_ENDPOINT
        );
    }

    #[test]
    fn test_config_mainnet_staging() {
        let canister = Canister::BitcoinMainnetStaging;
        assert_eq!(canister.network(), Network::BitcoinMainnet);
        assert_eq!(
            canister.canister_principal(),
            Principal::from_text(MAINNET_BITCOIN_STAGING_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            canister.get_canister_endpoint(),
            MAINNET_BITCOIN_STAGING_CANISTER_ENDPOINT
        );
    }

    #[test]
    fn test_config_testnet() {
        let canister = Canister::BitcoinTestnet;
        assert_eq!(canister.network(), Network::BitcoinTestnet);
        assert_eq!(
            canister.canister_principal(),
            Principal::from_text(TESTNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            canister.get_canister_endpoint(),
            TESTNET_BITCOIN_CANISTER_ENDPOINT
        );
    }

    #[test]
    fn test_config_dogecoin_mainnet() {
        let canister = Canister::DogecoinMainnet;
        assert_eq!(canister.network(), Network::DogecoinMainnet);
        assert_eq!(
            canister.canister_principal(),
            Principal::from_text(MAINNET_DOGECOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            canister.get_canister_endpoint(),
            MAINNET_DOGECOIN_CANISTER_ENDPOINT
        );
    }

    #[test]
    fn test_config_dogecoin_mainnet_staging() {
        let canister = Canister::DogecoinMainnetStaging;
        assert_eq!(canister.network(), Network::DogecoinMainnet);
        assert_eq!(
            canister.canister_principal(),
            Principal::from_text(MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            canister.get_canister_endpoint(),
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
            let config = StoredConfig::for_target(canister);
            let encoded = encode(&config);
            let decoded: StoredConfig = decode(&encoded);
            assert_eq!(config, decoded);
        }
    }
}
