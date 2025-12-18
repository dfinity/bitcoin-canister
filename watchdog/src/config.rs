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
}

/// Typed configuration with compile-time provider safety.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config<P: BlockProvider> {
    /// The canister provider to monitor.
    pub canister: P,

    /// The explorer providers to compare against.
    pub explorers: Vec<P>,

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

impl<P: BlockProvider> Config<P> {
    /// Returns all providers (explorers + canister).
    pub fn all_providers(&self) -> Vec<P>
    where
        P: Clone,
    {
        let mut providers = self.explorers.clone();
        providers.push(self.canister.clone());
        providers
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

impl Config<BitcoinMainnetProviderBlockApi> {
    pub fn bitcoin_mainnet() -> Self {
        Self {
            canister: BitcoinMainnetProviderBlockApi::BitcoinCanister,
            explorers: vec![
                BitcoinMainnetProviderBlockApi::ApiBitapsCom,
                BitcoinMainnetProviderBlockApi::ApiBlockchairCom,
                BitcoinMainnetProviderBlockApi::ApiBlockcypherCom,
                BitcoinMainnetProviderBlockApi::BlockchainInfo,
                BitcoinMainnetProviderBlockApi::BlockstreamInfo,
                BitcoinMainnetProviderBlockApi::Mempool,
            ],
            blocks_behind_threshold: 2,
            blocks_ahead_threshold: 2,
            min_explorers: 3,
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
        }
    }

    pub fn bitcoin_mainnet_staging() -> Self {
        Self {
            canister: BitcoinMainnetProviderBlockApi::BitcoinCanister,
            explorers: Self::bitcoin_mainnet().explorers,
            blocks_behind_threshold: 2,
            blocks_ahead_threshold: 2,
            min_explorers: 3,
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
        }
    }
}

impl Config<BitcoinTestnetProviderBlockApi> {
    pub fn bitcoin_testnet() -> Self {
        Self {
            canister: BitcoinTestnetProviderBlockApi::BitcoinCanister,
            explorers: vec![BitcoinTestnetProviderBlockApi::Mempool],
            blocks_behind_threshold: 1000,
            blocks_ahead_threshold: 1000,
            min_explorers: 1,
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
        }
    }
}

impl Config<DogecoinProviderBlockApi> {
    pub fn dogecoin_mainnet() -> Self {
        Self {
            canister: DogecoinProviderBlockApi::DogecoinCanister,
            explorers: vec![
                DogecoinProviderBlockApi::ApiBlockchairCom,
                DogecoinProviderBlockApi::ApiBlockcypherCom,
                DogecoinProviderBlockApi::TokenView,
            ],
            blocks_behind_threshold: 4,
            blocks_ahead_threshold: 4,
            min_explorers: 2,
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC,
        }
    }

    pub fn dogecoin_mainnet_staging() -> Self {
        Self {
            canister: DogecoinProviderBlockApi::DogecoinCanister,
            explorers: Self::dogecoin_mainnet().explorers,
            blocks_behind_threshold: 4,
            blocks_ahead_threshold: 4,
            min_explorers: 2,
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC,
        }
    }
}

/// Stored configuration (serializable form with strings).
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredConfig {
    /// The canister provider name.
    pub canister: String,

    /// The explorer provider names.
    pub explorers: Vec<String>,

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

impl<P: BlockProvider + Clone> From<&Config<P>> for StoredConfig {
    fn from(config: &Config<P>) -> Self {
        Self {
            canister: config.canister.to_string(),
            explorers: config.explorers.iter().map(|p| p.to_string()).collect(),
            blocks_behind_threshold: config.blocks_behind_threshold,
            blocks_ahead_threshold: config.blocks_ahead_threshold,
            min_explorers: config.min_explorers,
            delay_before_first_fetch_sec: config.delay_before_first_fetch_sec,
            interval_between_fetches_sec: config.interval_between_fetches_sec,
        }
    }
}

impl StoredConfig {
    /// Creates a default stored config for the given canister target.
    pub fn for_target(canister: Canister) -> Self {
        match canister {
            Canister::BitcoinMainnet => (&Config::bitcoin_mainnet()).into(),
            Canister::BitcoinMainnetStaging => (&Config::bitcoin_mainnet_staging()).into(),
            Canister::BitcoinTestnet => (&Config::bitcoin_testnet()).into(),
            Canister::DogecoinMainnet => (&Config::dogecoin_mainnet()).into(),
            Canister::DogecoinMainnetStaging => (&Config::dogecoin_mainnet_staging()).into(),
        }
    }

    /// Returns all providers (explorers + canister) parsed from stored strings.
    pub fn get_providers(&self, canister: Canister) -> Vec<Box<dyn BlockProvider>> {
        let all_names = self.explorers.iter().chain(std::iter::once(&self.canister));

        match canister {
            Canister::BitcoinMainnet | Canister::BitcoinMainnetStaging => all_names
                .filter_map(|s| s.parse::<BitcoinMainnetProviderBlockApi>().ok())
                .map(|p| Box::new(p) as Box<dyn BlockProvider>)
                .collect(),
            Canister::BitcoinTestnet => all_names
                .filter_map(|s| s.parse::<BitcoinTestnetProviderBlockApi>().ok())
                .map(|p| Box::new(p) as Box<dyn BlockProvider>)
                .collect(),
            Canister::DogecoinMainnet | Canister::DogecoinMainnetStaging => all_names
                .filter_map(|s| s.parse::<DogecoinProviderBlockApi>().ok())
                .map(|p| Box::new(p) as Box<dyn BlockProvider>)
                .collect(),
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

/// Configuration for the candid API get_config response.
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
    /// Combines configuration values from `Canister` with values from `StoredConfig`.
    pub fn from_parts(canister: Canister, config: StoredConfig) -> Self {
        Self {
            network: canister.network(),
            blocks_behind_threshold: config.blocks_behind_threshold,
            blocks_ahead_threshold: config.blocks_ahead_threshold,
            min_explorers: config.min_explorers,
            canister_principal: canister.canister_principal(),
            delay_before_first_fetch_sec: config.delay_before_first_fetch_sec,
            interval_between_fetches_sec: config.interval_between_fetches_sec,
            explorers: config.explorers,
            subnet_type: canister.subnet_type(),
        }
    }
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
    fn test_canister_network() {
        assert_eq!(Canister::BitcoinMainnet.network(), Network::BitcoinMainnet);
        assert_eq!(
            Canister::BitcoinMainnetStaging.network(),
            Network::BitcoinMainnet
        );
        assert_eq!(Canister::BitcoinTestnet.network(), Network::BitcoinTestnet);
        assert_eq!(
            Canister::DogecoinMainnet.network(),
            Network::DogecoinMainnet
        );
        assert_eq!(
            Canister::DogecoinMainnetStaging.network(),
            Network::DogecoinMainnet
        );
    }

    #[test]
    fn test_canister_principal() {
        assert_eq!(
            Canister::BitcoinMainnet.canister_principal(),
            Principal::from_text(MAINNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            Canister::BitcoinMainnetStaging.canister_principal(),
            Principal::from_text(MAINNET_BITCOIN_STAGING_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            Canister::BitcoinTestnet.canister_principal(),
            Principal::from_text(TESTNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            Canister::DogecoinMainnet.canister_principal(),
            Principal::from_text(MAINNET_DOGECOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(
            Canister::DogecoinMainnetStaging.canister_principal(),
            Principal::from_text(MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL).unwrap()
        );
    }

    #[test]
    fn test_canister_subnet_types() {
        assert_eq!(Canister::BitcoinMainnet.subnet_type(), SubnetType::System);
        assert_eq!(
            Canister::BitcoinMainnetStaging.subnet_type(),
            SubnetType::Application
        );
        assert_eq!(Canister::BitcoinTestnet.subnet_type(), SubnetType::System);
        assert_eq!(Canister::DogecoinMainnet.subnet_type(), SubnetType::System);
        assert_eq!(
            Canister::DogecoinMainnetStaging.subnet_type(),
            SubnetType::Application
        );
    }

    #[test]
    fn test_canister_endpoints() {
        assert_eq!(
            Canister::BitcoinMainnet.get_canister_endpoint(),
            MAINNET_BITCOIN_CANISTER_ENDPOINT
        );
        assert_eq!(
            Canister::BitcoinMainnetStaging.get_canister_endpoint(),
            MAINNET_BITCOIN_STAGING_CANISTER_ENDPOINT
        );
        assert_eq!(
            Canister::BitcoinTestnet.get_canister_endpoint(),
            TESTNET_BITCOIN_CANISTER_ENDPOINT
        );
        assert_eq!(
            Canister::DogecoinMainnet.get_canister_endpoint(),
            MAINNET_DOGECOIN_CANISTER_ENDPOINT
        );
        assert_eq!(
            Canister::DogecoinMainnetStaging.get_canister_endpoint(),
            MAINNET_DOGECOIN_STAGING_CANISTER_ENDPOINT
        );
    }

    #[test]
    fn test_config_bitcoin_mainnet() {
        let config = Config::bitcoin_mainnet();
        assert_eq!(
            config.canister,
            BitcoinMainnetProviderBlockApi::BitcoinCanister
        );
        assert_eq!(config.explorers.len(), 6);
        assert_eq!(config.all_providers().len(), 7);
        assert_eq!(config.blocks_behind_threshold, 2);
        assert_eq!(config.blocks_ahead_threshold, 2);
        assert_eq!(config.min_explorers, 3);
    }

    #[test]
    fn test_config_bitcoin_testnet() {
        let config = Config::bitcoin_testnet();
        assert_eq!(
            config.canister,
            BitcoinTestnetProviderBlockApi::BitcoinCanister
        );
        assert_eq!(config.explorers.len(), 1);
        assert_eq!(config.all_providers().len(), 2);
        assert_eq!(config.blocks_behind_threshold, 1000);
        assert_eq!(config.min_explorers, 1);
    }

    #[test]
    fn test_config_dogecoin_mainnet() {
        let config = Config::dogecoin_mainnet();
        assert_eq!(config.canister, DogecoinProviderBlockApi::DogecoinCanister);
        assert_eq!(config.explorers.len(), 3);
        assert_eq!(config.all_providers().len(), 4);
        assert_eq!(config.blocks_behind_threshold, 4);
        assert_eq!(config.min_explorers, 2);
    }

    #[test]
    fn test_stored_config_from_typed() {
        let typed = Config::bitcoin_mainnet();
        let stored: StoredConfig = (&typed).into();
        assert_eq!(stored.canister, "bitcoin_canister");
        assert_eq!(stored.explorers.len(), 6);
        assert_eq!(
            stored.blocks_behind_threshold,
            typed.blocks_behind_threshold
        );
    }

    #[test]
    fn test_stored_config_for_target() {
        let stored = StoredConfig::for_target(Canister::BitcoinMainnet);
        assert_eq!(stored.canister, "bitcoin_canister");
        assert_eq!(stored.explorers.len(), 6);

        let stored = StoredConfig::for_target(Canister::DogecoinMainnet);
        assert_eq!(stored.canister, "dogecoin_canister");
        assert_eq!(stored.explorers.len(), 3);
    }

    proptest! {
        #[test]
        fn test_stored_config_encode_decode(canister in prop_oneof![
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

    #[test]
    fn test_canister_storable_roundtrip() {
        for canister in [
            Canister::BitcoinMainnet,
            Canister::BitcoinMainnetStaging,
            Canister::BitcoinTestnet,
            Canister::DogecoinMainnet,
            Canister::DogecoinMainnetStaging,
        ] {
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
    fn test_stored_config_default() {
        let default = StoredConfig::default();
        let expected = StoredConfig::for_target(Canister::default());
        assert_eq!(default, expected);
    }

    #[test]
    fn test_candid_config_from_parts() {
        let canister = Canister::BitcoinMainnet;
        let stored = StoredConfig::for_target(canister);
        let candid = CandidConfig::from_parts(canister, stored.clone());

        assert_eq!(candid.network, Network::BitcoinMainnet);
        assert_eq!(
            candid.blocks_behind_threshold,
            stored.blocks_behind_threshold
        );
        assert_eq!(candid.blocks_ahead_threshold, stored.blocks_ahead_threshold);
        assert_eq!(candid.min_explorers, stored.min_explorers);
        assert_eq!(candid.canister_principal, canister.canister_principal());
        assert_eq!(candid.explorers.len(), 6);
        assert_eq!(candid.subnet_type, SubnetType::System);
    }
}
