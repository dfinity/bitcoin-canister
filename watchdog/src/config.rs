use crate::block_apis::{
    BitcoinMainnetExplorerBlockApi, BitcoinMainnetProviderBlockApi, BitcoinTestnetExplorerBlockApi,
    BitcoinTestnetProviderBlockApi, BlockProvider, DogecoinMainnetExplorerBlockApi,
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

/// Trait for canister configuration with strongly-typed providers.
pub trait CanisterConfig {
    type Provider: BlockProvider;

    /// Returns the canister API provider.
    fn canister() -> Self::Provider;

    /// Returns the list of explorer providers.
    fn explorers() -> Vec<Self::Provider>;

    /// Returns all providers (explorers + canister).
    fn all_providers() -> Vec<Self::Provider> {
        let mut providers = Self::explorers();
        providers.push(Self::canister());
        providers
    }

    /// Returns the network.
    fn network() -> Network;

    /// Returns the canister principal.
    fn canister_principal() -> Principal;

    /// Returns the subnet type.
    fn subnet_type() -> SubnetType;

    /// Returns the canister metrics endpoint.
    fn get_canister_endpoint() -> String {
        let principal = Self::canister_principal().to_text();
        let suffix = match Self::subnet_type() {
            SubnetType::System => "raw.ic0.app",
            SubnetType::Application => "raw.icp0.io",
        };
        format!("https://{principal}.{suffix}/metrics")
    }
}

/// Bitcoin mainnet canister configuration.
pub struct BitcoinMainnetCanister;

impl CanisterConfig for BitcoinMainnetCanister {
    type Provider = BitcoinMainnetProviderBlockApi;

    fn canister() -> Self::Provider {
        BitcoinMainnetProviderBlockApi::BitcoinCanister
    }

    fn explorers() -> Vec<Self::Provider> {
        vec![
            BitcoinMainnetProviderBlockApi::Mainnet(BitcoinMainnetExplorerBlockApi::ApiBitapsCom),
            BitcoinMainnetProviderBlockApi::Mainnet(
                BitcoinMainnetExplorerBlockApi::ApiBlockchairCom,
            ),
            BitcoinMainnetProviderBlockApi::Mainnet(
                BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom,
            ),
            BitcoinMainnetProviderBlockApi::Mainnet(BitcoinMainnetExplorerBlockApi::BlockchainInfo),
            BitcoinMainnetProviderBlockApi::Mainnet(
                BitcoinMainnetExplorerBlockApi::BlockstreamInfo,
            ),
            BitcoinMainnetProviderBlockApi::Mainnet(BitcoinMainnetExplorerBlockApi::Mempool),
        ]
    }

    fn network() -> Network {
        Network::BitcoinMainnet
    }

    fn canister_principal() -> Principal {
        Principal::from_text(MAINNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
    }

    fn subnet_type() -> SubnetType {
        SubnetType::System
    }
}

/// Bitcoin mainnet staging canister configuration.
pub struct BitcoinMainnetStagingCanister;

impl CanisterConfig for BitcoinMainnetStagingCanister {
    type Provider = BitcoinMainnetProviderBlockApi;

    fn canister() -> Self::Provider {
        BitcoinMainnetProviderBlockApi::BitcoinCanister
    }

    fn explorers() -> Vec<Self::Provider> {
        BitcoinMainnetCanister::explorers()
    }

    fn network() -> Network {
        Network::BitcoinMainnet
    }

    fn canister_principal() -> Principal {
        Principal::from_text(MAINNET_BITCOIN_STAGING_CANISTER_PRINCIPAL).unwrap()
    }

    fn subnet_type() -> SubnetType {
        SubnetType::Application
    }
}

/// Bitcoin testnet canister configuration.
pub struct BitcoinTestnetCanister;

impl CanisterConfig for BitcoinTestnetCanister {
    type Provider = BitcoinTestnetProviderBlockApi;

    fn canister() -> Self::Provider {
        BitcoinTestnetProviderBlockApi::BitcoinCanister
    }

    fn explorers() -> Vec<Self::Provider> {
        vec![BitcoinTestnetProviderBlockApi::Testnet(
            BitcoinTestnetExplorerBlockApi::Mempool,
        )]
    }

    fn network() -> Network {
        Network::BitcoinTestnet
    }

    fn canister_principal() -> Principal {
        Principal::from_text(TESTNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
    }

    fn subnet_type() -> SubnetType {
        SubnetType::System
    }
}

/// Dogecoin mainnet canister configuration.
pub struct DogecoinMainnetCanister;

impl CanisterConfig for DogecoinMainnetCanister {
    type Provider = DogecoinProviderBlockApi;

    fn canister() -> Self::Provider {
        DogecoinProviderBlockApi::DogecoinCanister
    }

    fn explorers() -> Vec<Self::Provider> {
        vec![
            DogecoinProviderBlockApi::Mainnet(DogecoinMainnetExplorerBlockApi::ApiBlockchairCom),
            DogecoinProviderBlockApi::Mainnet(DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom),
            DogecoinProviderBlockApi::Mainnet(DogecoinMainnetExplorerBlockApi::TokenView),
        ]
    }

    fn network() -> Network {
        Network::DogecoinMainnet
    }

    fn canister_principal() -> Principal {
        Principal::from_text(MAINNET_DOGECOIN_CANISTER_PRINCIPAL).unwrap()
    }

    fn subnet_type() -> SubnetType {
        SubnetType::System
    }
}

/// Dogecoin mainnet staging canister configuration.
pub struct DogecoinMainnetStagingCanister;

impl CanisterConfig for DogecoinMainnetStagingCanister {
    type Provider = DogecoinProviderBlockApi;

    fn canister() -> Self::Provider {
        DogecoinProviderBlockApi::DogecoinCanister
    }

    fn explorers() -> Vec<Self::Provider> {
        DogecoinMainnetCanister::explorers()
    }

    fn network() -> Network {
        Network::DogecoinMainnet
    }

    fn canister_principal() -> Principal {
        Principal::from_text(MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL).unwrap()
    }

    fn subnet_type() -> SubnetType {
        SubnetType::Application
    }
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
            Canister::BitcoinMainnet => BitcoinMainnetCanister::network(),
            Canister::BitcoinMainnetStaging => BitcoinMainnetStagingCanister::network(),
            Canister::BitcoinTestnet => BitcoinTestnetCanister::network(),
            Canister::DogecoinMainnet => DogecoinMainnetCanister::network(),
            Canister::DogecoinMainnetStaging => DogecoinMainnetStagingCanister::network(),
        }
    }

    pub fn canister_principal(&self) -> Principal {
        match self {
            Canister::BitcoinMainnet => BitcoinMainnetCanister::canister_principal(),
            Canister::BitcoinMainnetStaging => BitcoinMainnetStagingCanister::canister_principal(),
            Canister::BitcoinTestnet => BitcoinTestnetCanister::canister_principal(),
            Canister::DogecoinMainnet => DogecoinMainnetCanister::canister_principal(),
            Canister::DogecoinMainnetStaging => {
                DogecoinMainnetStagingCanister::canister_principal()
            }
        }
    }

    pub fn subnet_type(&self) -> SubnetType {
        match self {
            Canister::BitcoinMainnet => BitcoinMainnetCanister::subnet_type(),
            Canister::BitcoinMainnetStaging => BitcoinMainnetStagingCanister::subnet_type(),
            Canister::BitcoinTestnet => BitcoinTestnetCanister::subnet_type(),
            Canister::DogecoinMainnet => DogecoinMainnetCanister::subnet_type(),
            Canister::DogecoinMainnetStaging => DogecoinMainnetStagingCanister::subnet_type(),
        }
    }

    pub fn get_canister_endpoint(&self) -> String {
        match self {
            Canister::BitcoinMainnet => BitcoinMainnetCanister::get_canister_endpoint(),
            Canister::BitcoinMainnetStaging => {
                BitcoinMainnetStagingCanister::get_canister_endpoint()
            }
            Canister::BitcoinTestnet => BitcoinTestnetCanister::get_canister_endpoint(),
            Canister::DogecoinMainnet => DogecoinMainnetCanister::get_canister_endpoint(),
            Canister::DogecoinMainnetStaging => {
                DogecoinMainnetStagingCanister::get_canister_endpoint()
            }
        }
    }

    pub fn explorer_names(&self) -> Vec<String> {
        match self {
            Canister::BitcoinMainnet => BitcoinMainnetCanister::explorers()
                .iter()
                .map(|p| p.to_string())
                .collect(),
            Canister::BitcoinMainnetStaging => BitcoinMainnetStagingCanister::explorers()
                .iter()
                .map(|p| p.to_string())
                .collect(),
            Canister::BitcoinTestnet => BitcoinTestnetCanister::explorers()
                .iter()
                .map(|p| p.to_string())
                .collect(),
            Canister::DogecoinMainnet => DogecoinMainnetCanister::explorers()
                .iter()
                .map(|p| p.to_string())
                .collect(),
            Canister::DogecoinMainnetStaging => DogecoinMainnetStagingCanister::explorers()
                .iter()
                .map(|p| p.to_string())
                .collect(),
        }
    }

    pub fn canister_name(&self) -> String {
        match self {
            Canister::BitcoinMainnet => BitcoinMainnetCanister::canister().to_string(),
            Canister::BitcoinMainnetStaging => {
                BitcoinMainnetStagingCanister::canister().to_string()
            }
            Canister::BitcoinTestnet => BitcoinTestnetCanister::canister().to_string(),
            Canister::DogecoinMainnet => DogecoinMainnetCanister::canister().to_string(),
            Canister::DogecoinMainnetStaging => {
                DogecoinMainnetStagingCanister::canister().to_string()
            }
        }
    }
}

/// Stored configuration for modifiable values.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
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

impl Config {
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

impl Default for Config {
    fn default() -> Self {
        Config::for_target(Canister::default())
    }
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
    /// Combines configuration values from `Canister` with values from `Config`.
    pub fn from_parts(canister: Canister, config: Config) -> Self {
        Self {
            network: canister.network(),
            blocks_behind_threshold: config.blocks_behind_threshold,
            blocks_ahead_threshold: config.blocks_ahead_threshold,
            min_explorers: config.min_explorers,
            canister_principal: canister.canister_principal(),
            delay_before_first_fetch_sec: config.delay_before_first_fetch_sec,
            interval_between_fetches_sec: config.interval_between_fetches_sec,
            explorers: canister.explorer_names(),
            subnet_type: canister.subnet_type(),
        }
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
            let config = Config::for_target(canister);
            let encoded = encode(&config);
            let decoded: Config = decode(&encoded);
            assert_eq!(config, decoded);
        }
    }

    #[test]
    fn test_bitcoin_mainnet_canister_config() {
        assert_eq!(
            BitcoinMainnetCanister::canister(),
            BitcoinMainnetProviderBlockApi::BitcoinCanister
        );
        assert_eq!(BitcoinMainnetCanister::explorers().len(), 6);
        assert_eq!(BitcoinMainnetCanister::all_providers().len(), 7);
        assert_eq!(BitcoinMainnetCanister::network(), Network::BitcoinMainnet);
        assert_eq!(BitcoinMainnetCanister::subnet_type(), SubnetType::System);
    }

    #[test]
    fn test_bitcoin_mainnet_staging_canister_config() {
        assert_eq!(
            BitcoinMainnetStagingCanister::canister(),
            BitcoinMainnetProviderBlockApi::BitcoinCanister
        );
        // Staging uses same explorers as mainnet
        assert_eq!(
            BitcoinMainnetStagingCanister::explorers(),
            BitcoinMainnetCanister::explorers()
        );
        assert_eq!(
            BitcoinMainnetStagingCanister::network(),
            Network::BitcoinMainnet
        );
        assert_eq!(
            BitcoinMainnetStagingCanister::subnet_type(),
            SubnetType::Application
        );
    }

    #[test]
    fn test_bitcoin_testnet_canister_config() {
        assert_eq!(
            BitcoinTestnetCanister::canister(),
            BitcoinTestnetProviderBlockApi::BitcoinCanister
        );
        assert_eq!(BitcoinTestnetCanister::explorers().len(), 1);
        assert_eq!(BitcoinTestnetCanister::all_providers().len(), 2);
        assert_eq!(BitcoinTestnetCanister::network(), Network::BitcoinTestnet);
        assert_eq!(BitcoinTestnetCanister::subnet_type(), SubnetType::System);
    }

    #[test]
    fn test_dogecoin_mainnet_canister_config() {
        assert_eq!(
            DogecoinMainnetCanister::canister(),
            DogecoinProviderBlockApi::DogecoinCanister
        );
        assert_eq!(DogecoinMainnetCanister::explorers().len(), 3);
        assert_eq!(DogecoinMainnetCanister::all_providers().len(), 4);
        assert_eq!(DogecoinMainnetCanister::network(), Network::DogecoinMainnet);
        assert_eq!(DogecoinMainnetCanister::subnet_type(), SubnetType::System);
    }

    #[test]
    fn test_dogecoin_mainnet_staging_canister_config() {
        assert_eq!(
            DogecoinMainnetStagingCanister::canister(),
            DogecoinProviderBlockApi::DogecoinCanister
        );
        // Staging uses same explorers as mainnet
        assert_eq!(
            DogecoinMainnetStagingCanister::explorers(),
            DogecoinMainnetCanister::explorers()
        );
        assert_eq!(
            DogecoinMainnetStagingCanister::network(),
            Network::DogecoinMainnet
        );
        assert_eq!(
            DogecoinMainnetStagingCanister::subnet_type(),
            SubnetType::Application
        );
    }

    #[test]
    fn test_canister_explorer_names() {
        let expected_bitcoin_mainnet = vec![
            "bitcoin_api_bitaps_com_mainnet",
            "bitcoin_api_blockchair_com_mainnet",
            "bitcoin_api_blockcypher_com_mainnet",
            "bitcoin_blockchain_info_mainnet",
            "bitcoin_blockstream_info_mainnet",
            "bitcoin_mempool_mainnet",
        ];
        assert_eq!(
            Canister::BitcoinMainnet.explorer_names(),
            expected_bitcoin_mainnet
        );
        assert_eq!(
            Canister::BitcoinMainnetStaging.explorer_names(),
            expected_bitcoin_mainnet
        );

        assert_eq!(
            Canister::BitcoinTestnet.explorer_names(),
            vec!["bitcoin_mempool_testnet"]
        );

        let expected_dogecoin = vec![
            "dogecoin_api_blockchair_com_mainnet",
            "dogecoin_api_blockcypher_com_mainnet",
            "dogecoin_tokenview_mainnet",
        ];
        assert_eq!(
            Canister::DogecoinMainnet.explorer_names(),
            expected_dogecoin
        );
        assert_eq!(
            Canister::DogecoinMainnetStaging.explorer_names(),
            expected_dogecoin
        );
    }

    #[test]
    fn test_canister_names() {
        assert_eq!(Canister::BitcoinMainnet.canister_name(), "bitcoin_canister");
        assert_eq!(
            Canister::BitcoinMainnetStaging.canister_name(),
            "bitcoin_canister"
        );
        assert_eq!(Canister::BitcoinTestnet.canister_name(), "bitcoin_canister");
        assert_eq!(
            Canister::DogecoinMainnet.canister_name(),
            "dogecoin_canister"
        );
        assert_eq!(
            Canister::DogecoinMainnetStaging.canister_name(),
            "dogecoin_canister"
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
    fn test_candid_config_from_parts() {
        let canister = Canister::BitcoinMainnet;
        let config = Config::for_target(canister);
        let candid = CandidConfig::from_parts(canister, config.clone());

        assert_eq!(candid.network, Network::BitcoinMainnet);
        assert_eq!(
            candid.blocks_behind_threshold,
            config.blocks_behind_threshold
        );
        assert_eq!(candid.blocks_ahead_threshold, config.blocks_ahead_threshold);
        assert_eq!(candid.min_explorers, config.min_explorers);
        assert_eq!(
            candid.canister_principal,
            Principal::from_text(MAINNET_BITCOIN_CANISTER_PRINCIPAL).unwrap()
        );
        assert_eq!(candid.explorers.len(), 6);
        assert_eq!(candid.subnet_type, SubnetType::System);
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
    fn test_config_default() {
        let default = Config::default();
        let expected = Config::for_target(Canister::default());
        assert_eq!(default, expected);
    }
}
