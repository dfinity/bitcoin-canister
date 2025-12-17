use crate::block_apis::{
    BitcoinMainnetExplorerBlockApi, BitcoinMainnetProviderBlockApi, BitcoinTestnetExplorerBlockApi,
    BitcoinTestnetProviderBlockApi, BlockApi, BlockApiTrait, DogecoinMainnetExplorerBlockApi,
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config<P: BlockApiTrait> {
    /// The network to use.
    pub network: Network,

    /// The canister to monitor.
    pub canister: P,

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
    pub explorers: Vec<P>,

    /// Type of subnet on which the watchdog and target canisters are deployed.
    pub subnet_type: SubnetType,
}

pub type BitcoinMainnetConfig = Config<BitcoinMainnetProviderBlockApi>;
pub type BitcoinTestnetConfig = Config<BitcoinTestnetProviderBlockApi>;
pub type DogecoinMainnetConfig = Config<DogecoinProviderBlockApi>;

impl<P: BlockApiTrait> Config<P> {
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

impl BitcoinMainnetConfig {
    /// Configuration for Bitcoin mainnet canister.
    pub fn for_prod() -> Self {
        Self {
            network: Network::BitcoinMainnet,
            canister: BitcoinMainnetProviderBlockApi::BitcoinCanister,
            blocks_behind_threshold: 2,
            blocks_ahead_threshold: 2,
            min_explorers: 3,
            canister_principal: Principal::from_text(MAINNET_BITCOIN_CANISTER_PRINCIPAL).unwrap(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBitapsCom,
                ),
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBlockchairCom,
                ),
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom,
                ),
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockchainInfo,
                ),
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockstreamInfo,
                ),
                BitcoinMainnetProviderBlockApi::Mainnet(BitcoinMainnetExplorerBlockApi::Mempool),
            ],
            subnet_type: SubnetType::System,
        }
    }

    /// Configuration for Bitcoin mainnet staging canister.
    pub fn for_staging() -> Self {
        Self {
            network: Network::BitcoinMainnet,
            canister: BitcoinMainnetProviderBlockApi::BitcoinCanister,
            blocks_behind_threshold: 2,
            blocks_ahead_threshold: 2,
            min_explorers: 3,
            canister_principal: Principal::from_text(MAINNET_BITCOIN_STAGING_CANISTER_PRINCIPAL)
                .unwrap(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBitapsCom,
                ),
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBlockchairCom,
                ),
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom,
                ),
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockchainInfo,
                ),
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockstreamInfo,
                ),
                BitcoinMainnetProviderBlockApi::Mainnet(BitcoinMainnetExplorerBlockApi::Mempool),
            ],
            subnet_type: SubnetType::Application,
        }
    }
}

impl BitcoinTestnetConfig {
    /// Configuration for Bitcoin testnet canister.
    pub fn for_prod() -> Self {
        Self {
            network: Network::BitcoinTestnet,
            canister: BitcoinTestnetProviderBlockApi::BitcoinCanister,
            blocks_behind_threshold: 1000,
            blocks_ahead_threshold: 1000,
            min_explorers: 1,
            canister_principal: Principal::from_text(TESTNET_BITCOIN_CANISTER_PRINCIPAL).unwrap(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: BITCOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![BitcoinTestnetProviderBlockApi::Testnet(
                BitcoinTestnetExplorerBlockApi::Mempool,
            )],
            subnet_type: SubnetType::System,
        }
    }
}

impl DogecoinMainnetConfig {
    /// Configuration for Dogecoin mainnet canister.
    pub fn for_prod() -> Self {
        Self {
            network: Network::DogecoinMainnet,
            canister: DogecoinProviderBlockApi::DogecoinCanister,
            blocks_behind_threshold: 4,
            blocks_ahead_threshold: 4,
            min_explorers: 2,
            canister_principal: Principal::from_text(MAINNET_DOGECOIN_CANISTER_PRINCIPAL).unwrap(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![
                DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::ApiBlockchairCom,
                ),
                DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom,
                ),
                DogecoinProviderBlockApi::Mainnet(DogecoinMainnetExplorerBlockApi::TokenView),
            ],
            subnet_type: SubnetType::System,
        }
    }

    /// Configuration for Dogecoin mainnet staging canister.
    pub fn for_staging() -> Self {
        Self {
            network: Network::DogecoinMainnet,
            canister: DogecoinProviderBlockApi::DogecoinCanister,
            blocks_behind_threshold: 4,
            blocks_ahead_threshold: 4,
            min_explorers: 2,
            canister_principal: Principal::from_text(MAINNET_DOGECOIN_STAGING_CANISTER_PRINCIPAL)
                .unwrap(),
            delay_before_first_fetch_sec: DELAY_BEFORE_FIRST_FETCH_SEC,
            interval_between_fetches_sec: DOGECOIN_INTERVAL_BETWEEN_FETCHES_SEC,
            explorers: vec![
                DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::ApiBlockchairCom,
                ),
                DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom,
                ),
                DogecoinProviderBlockApi::Mainnet(DogecoinMainnetExplorerBlockApi::TokenView),
            ],
            subnet_type: SubnetType::Application,
        }
    }
}

/// Stored configuration that uses type-erased BlockApi for serialization.
/// This is used for stable storage since we can't store generic types.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredConfig {
    /// The network to use.
    pub network: Network,

    /// The canister to monitor.
    pub canister: BlockApi,

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

    /// Type of subnet on which the watchdog and target canisters are deployed.
    pub subnet_type: SubnetType,
}

impl StoredConfig {
    /// Creates a new configuration for the given canister.
    pub fn for_target(canister: Canister) -> Self {
        match canister {
            Canister::BitcoinMainnet => BitcoinMainnetConfig::for_prod().into(),
            Canister::BitcoinMainnetStaging => BitcoinMainnetConfig::for_staging().into(),
            Canister::BitcoinTestnet => BitcoinTestnetConfig::for_prod().into(),
            Canister::DogecoinMainnet => DogecoinMainnetConfig::for_prod().into(),
            Canister::DogecoinMainnetStaging => DogecoinMainnetConfig::for_staging().into(),
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

impl Default for StoredConfig {
    fn default() -> Self {
        StoredConfig::for_target(Canister::BitcoinMainnet)
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

impl<P: BlockApiTrait + Into<BlockApi>> From<Config<P>> for StoredConfig {
    fn from(config: Config<P>) -> Self {
        Self {
            network: config.network,
            canister: config.canister.into(),
            blocks_behind_threshold: config.blocks_behind_threshold,
            blocks_ahead_threshold: config.blocks_ahead_threshold,
            min_explorers: config.min_explorers,
            canister_principal: config.canister_principal,
            delay_before_first_fetch_sec: config.delay_before_first_fetch_sec,
            interval_between_fetches_sec: config.interval_between_fetches_sec,
            explorers: config.explorers.into_iter().map(Into::into).collect(),
            subnet_type: config.subnet_type,
        }
    }
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
        let config = BitcoinMainnetConfig::for_prod();
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
        let config = BitcoinMainnetConfig::for_staging();
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
        let config = BitcoinTestnetConfig::for_prod();
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
        let config = DogecoinMainnetConfig::for_prod();
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
        let config = DogecoinMainnetConfig::for_staging();
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
            let config = StoredConfig::for_target(canister);
            let encoded = encode(&config);
            let decoded: StoredConfig = decode(&encoded);
            assert_eq!(config, decoded);
        }
    }
}
