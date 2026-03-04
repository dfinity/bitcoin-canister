use crate::block_apis::{
    BitcoinMainnetProviderBlockApi, BitcoinTestnetProviderBlockApi, BlockProvider,
    DogecoinProviderBlockApi,
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
const DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC: u64 = 60;

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

/// Canister to monitor (stored in stable memory).
#[derive(Copy, Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Canister {
    #[serde(rename = "bitcoin_mainnet")]
    #[default]
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
            Canister::DogecoinMainnet | Canister::DogecoinMainnetStaging => {
                Network::DogecoinMainnet
            }
        }
    }

    pub fn canister_principal(&self) -> Principal {
        let principal_str = match self {
            Canister::BitcoinMainnet => MAINNET_BITCOIN_CANISTER_PRINCIPAL,
            Canister::BitcoinMainnetStaging => MAINNET_BITCOIN_STAGING_CANISTER_PRINCIPAL,
            Canister::BitcoinTestnet => TESTNET_BITCOIN_CANISTER_PRINCIPAL,
            Canister::DogecoinMainnet => MAINNET_DOGECOIN_CANISTER_PRINCIPAL,
            Canister::DogecoinMainnetStaging => MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL,
        };
        Principal::from_text(principal_str).unwrap()
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

    pub fn get_canister_endpoint(&self) -> String {
        let principal = self.canister_principal().to_text();
        let suffix = match self.subnet_type() {
            SubnetType::System => "raw.ic0.app",
            SubnetType::Application => "raw.icp0.io",
        };
        format!("https://{principal}.{suffix}/metrics")
    }

    /// Returns the canister provider.
    pub fn provider(&self) -> Box<dyn BlockProvider> {
        match self {
            Canister::BitcoinMainnet | Canister::BitcoinMainnetStaging => {
                Box::new(BitcoinMainnetProviderBlockApi::BitcoinCanister)
            }
            Canister::BitcoinTestnet => Box::new(BitcoinTestnetProviderBlockApi::BitcoinCanister),
            Canister::DogecoinMainnet | Canister::DogecoinMainnetStaging => {
                Box::new(DogecoinProviderBlockApi::DogecoinCanister)
            }
        }
    }
}

/// Stored configuration.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
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

impl Config {
    /// Creates a default config for the given canister target.
    pub fn for_target(canister: Canister) -> Self {
        match canister {
            Canister::BitcoinMainnet | Canister::BitcoinMainnetStaging => Self {
                network: canister.network(),
                canister_principal: canister.canister_principal(),
                subnet_type: canister.subnet_type(),
                explorers: [
                    BitcoinMainnetProviderBlockApi::ApiBitapsCom,
                    BitcoinMainnetProviderBlockApi::ApiBlockchairCom,
                    BitcoinMainnetProviderBlockApi::ApiBlockcypherCom,
                    BitcoinMainnetProviderBlockApi::BlockchainInfo,
                    BitcoinMainnetProviderBlockApi::BlockstreamInfo,
                    BitcoinMainnetProviderBlockApi::Mempool,
                ]
                .iter()
                .map(|p| p.to_string())
                .collect(),
                blocks_behind_threshold: 2,
                blocks_ahead_threshold: 2,
                min_explorers: 3,
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            },
            Canister::BitcoinTestnet => Self {
                network: canister.network(),
                canister_principal: canister.canister_principal(),
                subnet_type: canister.subnet_type(),
                explorers: [BitcoinTestnetProviderBlockApi::Mempool]
                    .iter()
                    .map(|p| p.to_string())
                    .collect(),
                blocks_behind_threshold: 1000,
                blocks_ahead_threshold: 1000,
                min_explorers: 1,
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            },
            Canister::DogecoinMainnet | Canister::DogecoinMainnetStaging => Self {
                network: canister.network(),
                canister_principal: canister.canister_principal(),
                subnet_type: canister.subnet_type(),
                explorers: [
                    DogecoinProviderBlockApi::ApiBlockchairCom,
                    DogecoinProviderBlockApi::ApiBlockcypherCom,
                    DogecoinProviderBlockApi::PsyProtocol,
                ]
                .iter()
                .map(|p| p.to_string())
                .collect(),
                blocks_behind_threshold: 4,
                blocks_ahead_threshold: 4,
                min_explorers: 2,
                delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
                interval_between_fetches_sec: DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            },
        }
    }

    /// Returns all providers (explorers + canister) parsed from stored strings.
    pub fn get_providers(&self, canister: Canister) -> Vec<Box<dyn BlockProvider>> {
        let explorers: Vec<Box<dyn BlockProvider>> = match canister {
            Canister::BitcoinMainnet | Canister::BitcoinMainnetStaging => self
                .explorers
                .iter()
                .filter_map(|s| s.parse::<BitcoinMainnetProviderBlockApi>().ok())
                .map(|p| Box::new(p) as Box<dyn BlockProvider>)
                .collect(),
            Canister::BitcoinTestnet => self
                .explorers
                .iter()
                .filter_map(|s| s.parse::<BitcoinTestnetProviderBlockApi>().ok())
                .map(|p| Box::new(p) as Box<dyn BlockProvider>)
                .collect(),
            Canister::DogecoinMainnet | Canister::DogecoinMainnetStaging => self
                .explorers
                .iter()
                .filter_map(|s| s.parse::<DogecoinProviderBlockApi>().ok())
                .map(|p| Box::new(p) as Box<dyn BlockProvider>)
                .collect(),
        };
        explorers
            .into_iter()
            .chain(std::iter::once(canister.provider()))
            .collect()
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

impl Default for Config {
    fn default() -> Self {
        Config::for_target(Canister::default())
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

fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    let mut buf = vec![];
    ciborium::ser::into_writer(value, &mut buf).expect("failed to encode state");
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

    const ALL_CANISTERS: [Canister; 5] = [
        Canister::BitcoinMainnet,
        Canister::BitcoinMainnetStaging,
        Canister::BitcoinTestnet,
        Canister::DogecoinMainnet,
        Canister::DogecoinMainnetStaging,
    ];

    #[test]
    fn test_canister_endpoint_contains_principal() {
        for canister in ALL_CANISTERS {
            let endpoint = canister.get_canister_endpoint();
            let principal = canister.canister_principal().to_text();
            assert!(
                endpoint.contains(&principal),
                "Endpoint {} should contain principal {}",
                endpoint,
                principal
            );
        }
    }

    #[test]
    fn test_canister_provider() {
        for canister in ALL_CANISTERS {
            let provider_name = canister.provider().name();
            match canister {
                Canister::BitcoinMainnet
                | Canister::BitcoinMainnetStaging
                | Canister::BitcoinTestnet => {
                    assert_eq!(provider_name, "bitcoin_canister");
                }
                Canister::DogecoinMainnet | Canister::DogecoinMainnetStaging => {
                    assert_eq!(provider_name, "dogecoin_canister");
                }
            }
        }
    }

    #[test]
    fn test_staging_canisters_use_application_subnet() {
        assert_eq!(
            Canister::BitcoinMainnetStaging.subnet_type(),
            SubnetType::Application
        );
        assert_eq!(
            Canister::DogecoinMainnetStaging.subnet_type(),
            SubnetType::Application
        );
    }

    #[test]
    fn test_production_canisters_use_system_subnet() {
        assert_eq!(Canister::BitcoinMainnet.subnet_type(), SubnetType::System);
        assert_eq!(Canister::BitcoinTestnet.subnet_type(), SubnetType::System);
        assert_eq!(Canister::DogecoinMainnet.subnet_type(), SubnetType::System);
    }

    #[test]
    fn test_staging_canisters_have_different_principals() {
        assert_ne!(
            Canister::BitcoinMainnet.canister_principal(),
            Canister::BitcoinMainnetStaging.canister_principal()
        );
        assert_ne!(
            Canister::DogecoinMainnet.canister_principal(),
            Canister::DogecoinMainnetStaging.canister_principal()
        );
    }

    #[test]
    fn test_canister_storable_roundtrip() {
        for canister in ALL_CANISTERS {
            let bytes = canister.to_bytes();
            let decoded = Canister::from_bytes(bytes);
            assert_eq!(canister, decoded);
        }
    }

    #[test]
    fn test_canister_default() {
        assert_eq!(Canister::default(), Canister::BitcoinMainnet);
    }

    #[test]
    fn test_config_for_target() {
        for canister in ALL_CANISTERS {
            let config = Config::for_target(canister);
            assert_eq!(config.network, canister.network());
            assert_eq!(config.canister_principal, canister.canister_principal());
            assert_eq!(config.subnet_type, canister.subnet_type());
            assert!(!config.explorers.is_empty());

            // Verify get_providers includes canister provider
            let providers = config.get_providers(canister);
            assert_eq!(providers.len(), config.explorers.len() + 1);
            let provider_names: Vec<String> = providers.iter().map(|p| p.name()).collect();
            assert!(provider_names.contains(&canister.provider().name()));
        }
    }

    #[test]
    fn test_config_default() {
        assert_eq!(Config::default(), Config::for_target(Canister::default()));
    }

    #[test]
    fn test_config_thresholds() {
        let config = Config::for_target(Canister::BitcoinMainnet);
        assert_eq!(
            config.get_blocks_behind_threshold(),
            -(config.blocks_behind_threshold as i64)
        );
        assert_eq!(
            config.get_blocks_ahead_threshold(),
            config.blocks_ahead_threshold as i64
        );
    }

    proptest! {
        #[test]
        fn test_config_storable_roundtrip(canister in prop_oneof![
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
