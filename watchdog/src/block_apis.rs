use crate::endpoints::*;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use strum::{Display, EnumIter, IntoEnumIterator};

pub trait BlockApiTrait: Clone + Sized {
    async fn fetch_data(&self) -> serde_json::Value;
    fn network_canister() -> Self;
    /// Returns the list of all API providers.
    fn network_providers() -> Vec<Self>;
    fn network_explorers() -> Vec<Self>;
}

/// APIs that serve Bitcoin block data.
#[derive(
    Clone, Debug, Eq, PartialEq, Hash, CandidType, Serialize, Deserialize, PartialOrd, Ord,
)]
pub enum BitcoinBlockApi {
    #[serde(rename = "api_bitaps_com_mainnet")]
    ApiBitapsComMainnet,

    #[serde(rename = "api_blockchair_com_mainnet")]
    ApiBlockchairComMainnet,

    #[serde(rename = "api_blockcypher_com_mainnet")]
    ApiBlockcypherComMainnet,

    #[serde(rename = "bitcoin_canister")]
    BitcoinCanister, // Not an explorer.

    #[serde(rename = "blockchain_info_mainnet")]
    BlockchainInfoMainnet,

    #[serde(rename = "blockstream_info_mainnet")]
    BlockstreamInfoMainnet,

    #[serde(rename = "mempool_mainnet")]
    MempoolMainnet,

    #[serde(rename = "mempool_testnet")]
    MempoolTestnet,
}

impl std::fmt::Display for BitcoinBlockApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Extract the name from the JSON representation provided by serde-rename.
        let s = serde_json::to_string(&json!(self)).unwrap();
        let name = s
            .strip_prefix('\"')
            .and_then(|s| s.strip_suffix('\"'))
            .unwrap();

        write!(f, "{}", name)
    }
}

/// APIs that serve block data.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Display)]
pub enum BlockApi {
    #[strum(transparent)]
    BitcoinMainnetProvider(BitcoinMainnetProviderBlockApi),
    #[strum(transparent)]
    BitcoinTestnetProvider(BitcoinTestnetProviderBlockApi),
    #[strum(transparent)]
    DogecoinProvider(DogecoinProviderBlockApi),
}

impl BlockApi {
    async fn fetch_data(&self) -> serde_json::Value {
        match self {
            BlockApi::BitcoinMainnetProvider(api) => api.fetch_data().await,
            BlockApi::BitcoinTestnetProvider(api) => api.fetch_data().await,
            BlockApi::DogecoinProvider(api) => api.fetch_data().await,
        }
    }
}

/// Providers that serve Bitcoin block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Display)]
pub enum BitcoinMainnetProviderBlockApi {
    #[strum(serialize = "bitcoin_canister")]
    BitcoinCanister,
    #[strum(transparent)]
    Mainnet(BitcoinMainnetExplorerBlockApi),
}

impl BlockApiTrait for BitcoinMainnetProviderBlockApi {
    async fn fetch_data(&self) -> serde_json::Value {
        match self {
            Self::BitcoinCanister => endpoint_bitcoin_canister().send_request_json().await,
            Self::Mainnet(api) => match api {
                BitcoinMainnetExplorerBlockApi::ApiBitapsCom => {
                    endpoint_api_bitaps_com_block_mainnet()
                        .send_request_json()
                        .await
                }
                BitcoinMainnetExplorerBlockApi::ApiBlockchairCom => {
                    endpoint_api_blockchair_com_block_mainnet()
                        .send_request_json()
                        .await
                }
                BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom => {
                    endpoint_api_blockcypher_com_block_mainnet()
                        .send_request_json()
                        .await
                }
                BitcoinMainnetExplorerBlockApi::BlockchainInfo => {
                    let height_config = endpoint_blockchain_info_height_mainnet();
                    let hash_config = endpoint_blockchain_info_hash_mainnet();
                    let futures = vec![
                        height_config.send_request_json(),
                        hash_config.send_request_json(),
                    ];
                    let results = futures::future::join_all(futures).await;
                    match (results[0]["height"].as_u64(), results[1]["hash"].as_str()) {
                        (Some(height), Some(hash)) => {
                            json!({
                                "height": height,
                                "hash": hash,
                            })
                        }
                        _ => json!({}),
                    }
                }
                BitcoinMainnetExplorerBlockApi::BlockstreamInfo => {
                    let height_config = endpoint_blockstream_info_height_mainnet();
                    let hash_config = endpoint_blockstream_info_hash_mainnet();
                    let futures = vec![
                        height_config.send_request_json(),
                        hash_config.send_request_json(),
                    ];
                    let results = futures::future::join_all(futures).await;
                    match (results[0]["height"].as_u64(), results[1]["hash"].as_str()) {
                        (Some(height), Some(hash)) => {
                            json!({
                                "height": height,
                                "hash": hash,
                            })
                        }
                        _ => json!({}),
                    }
                }
                BitcoinMainnetExplorerBlockApi::Mempool => {
                    endpoint_mempool_height_mainnet().send_request_json().await
                }
            },
        }
    }

    fn network_canister() -> Self {
        Self::BitcoinCanister
    }

    fn network_providers() -> Vec<Self> {
        let mut providers: Vec<BitcoinMainnetProviderBlockApi> = Self::network_explorers();
        // Add the Bitcoin canister, since it's not an explorer.
        providers.push(BitcoinMainnetProviderBlockApi::BitcoinCanister);

        providers
    }

    fn network_explorers() -> Vec<Self> {
        let mut explorers: Vec<BitcoinMainnetProviderBlockApi> =
            BitcoinMainnetExplorerBlockApi::iter()
                .map(BitcoinMainnetProviderBlockApi::Mainnet)
                .collect();
        // Remove the explorers that are not configured.
        let configured: BTreeSet<_> = crate::storage::get_config().explorers.into_iter().collect();
        explorers
            .retain(|&x| configured.contains(&BlockApi::BitcoinMainnetProvider(x).to_string()));

        explorers
    }
}

impl From<BitcoinMainnetProviderBlockApi> for BlockApi {
    fn from(api: BitcoinMainnetProviderBlockApi) -> Self {
        BlockApi::BitcoinMainnetProvider(api)
    }
}

/// Explorers that serve Bitcoin mainnet block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, EnumIter, Display)]
pub enum BitcoinMainnetExplorerBlockApi {
    #[strum(serialize = "bitcoin_api_bitaps_com_mainnet")]
    ApiBitapsCom,
    #[strum(serialize = "bitcoin_api_blockchair_com_mainnet")]
    ApiBlockchairCom,
    #[strum(serialize = "bitcoin_api_blockcypher_com_mainnet")]
    ApiBlockcypherCom,
    #[strum(serialize = "bitcoin_blockchain_info_mainnet")]
    BlockchainInfo,
    #[strum(serialize = "bitcoin_blockstream_info_mainnet")]
    BlockstreamInfo,
    #[strum(serialize = "bitcoin_mempool_mainnet")]
    Mempool,
}

impl From<BitcoinMainnetExplorerBlockApi> for BlockApi {
    fn from(api: BitcoinMainnetExplorerBlockApi) -> Self {
        BlockApi::BitcoinMainnetProvider(BitcoinMainnetProviderBlockApi::Mainnet(api))
    }
}

/// Providers that serve testnet Bitcoin block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Display)]
pub enum BitcoinTestnetProviderBlockApi {
    #[strum(serialize = "bitcoin_canister")]
    BitcoinCanister,
    #[strum(transparent)]
    Testnet(BitcoinTestnetExplorerBlockApi),
}

impl BlockApiTrait for BitcoinTestnetProviderBlockApi {
    async fn fetch_data(&self) -> serde_json::Value {
        match self {
            Self::BitcoinCanister => endpoint_bitcoin_canister().send_request_json().await,
            Self::Testnet(api) => match api {
                BitcoinTestnetExplorerBlockApi::Mempool => {
                    endpoint_mempool_height_testnet().send_request_json().await
                }
            },
        }
    }

    fn network_canister() -> Self {
        Self::BitcoinCanister
    }

    fn network_providers() -> Vec<Self> {
        let mut providers = Self::network_explorers();
        // Add the Bitcoin canister, since it's not an explorer.
        providers.push(BitcoinTestnetProviderBlockApi::BitcoinCanister);

        providers
    }

    fn network_explorers() -> Vec<Self> {
        let mut explorers: Vec<BitcoinTestnetProviderBlockApi> =
            BitcoinTestnetExplorerBlockApi::iter()
                .map(BitcoinTestnetProviderBlockApi::Testnet)
                .collect();
        // Remove the explorers that are not configured.
        let configured: BTreeSet<_> = crate::storage::get_config().explorers.into_iter().collect();
        explorers
            .retain(|&x| configured.contains(&BlockApi::BitcoinTestnetProvider(x).to_string()));

        explorers
    }
}

impl From<BitcoinTestnetProviderBlockApi> for BlockApi {
    fn from(api: BitcoinTestnetProviderBlockApi) -> Self {
        BlockApi::BitcoinTestnetProvider(api)
    }
}

/// Explorers that serve Bitcoin testnet block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, EnumIter, Display)]
pub enum BitcoinTestnetExplorerBlockApi {
    #[strum(serialize = "bitcoin_mempool_testnet")]
    Mempool,
}

impl From<BitcoinTestnetExplorerBlockApi> for BlockApi {
    fn from(api: BitcoinTestnetExplorerBlockApi) -> Self {
        BlockApi::BitcoinTestnetProvider(BitcoinTestnetProviderBlockApi::Testnet(api))
    }
}

/// Providers that serve Dogecoin block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Display)]
pub enum DogecoinProviderBlockApi {
    #[strum(serialize = "dogecoin_canister")]
    DogecoinCanister,
    #[strum(transparent)]
    Mainnet(DogecoinMainnetExplorerBlockApi),
}

impl BlockApiTrait for DogecoinProviderBlockApi {
    async fn fetch_data(&self) -> serde_json::Value {
        match self {
            Self::DogecoinCanister => endpoint_dogecoin_canister().send_request_json().await,
            Self::Mainnet(api) => match api {
                DogecoinMainnetExplorerBlockApi::ApiBlockchairCom => {
                    endpoint_dogecoin_api_blockchair_com_block_mainnet()
                        .send_request_json()
                        .await
                }
                DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom => {
                    endpoint_dogecoin_api_blockcypher_com_block_mainnet()
                        .send_request_json()
                        .await
                }
                DogecoinMainnetExplorerBlockApi::TokenView => {
                    endpoint_dogecoin_tokenview_height_mainnet()
                        .send_request_json()
                        .await
                }
            },
        }
    }

    fn network_canister() -> Self {
        Self::DogecoinCanister
    }

    fn network_providers() -> Vec<Self> {
        let mut providers: Vec<DogecoinProviderBlockApi> = Self::network_explorers();
        // Add the Dogecoin canister, since it's not an explorer.
        providers.push(DogecoinProviderBlockApi::DogecoinCanister);

        providers
    }

    fn network_explorers() -> Vec<Self> {
        let mut explorers: Vec<DogecoinProviderBlockApi> = DogecoinMainnetExplorerBlockApi::iter()
            .map(DogecoinProviderBlockApi::Mainnet)
            .collect();
        // Remove the explorers that are not configured.
        let configured: BTreeSet<_> = crate::storage::get_config().explorers.into_iter().collect();
        explorers.retain(|&x| configured.contains(&BlockApi::DogecoinProvider(x).to_string()));

        explorers
    }
}

impl From<DogecoinProviderBlockApi> for BlockApi {
    fn from(api: DogecoinProviderBlockApi) -> Self {
        BlockApi::DogecoinProvider(api)
    }
}

/// Explorers that serve Dogecoin mainnet block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, EnumIter, Display)]
pub enum DogecoinMainnetExplorerBlockApi {
    #[strum(serialize = "dogecoin_api_blockchair_com_mainnet")]
    ApiBlockchairCom,
    #[strum(serialize = "dogecoin_api_blockcypher_com_mainnet")]
    ApiBlockcypherCom,
    #[strum(serialize = "dogecoin_tokenview_mainnet")]
    TokenView,
}

impl From<DogecoinMainnetExplorerBlockApi> for BlockApi {
    fn from(api: DogecoinMainnetExplorerBlockApi) -> Self {
        BlockApi::DogecoinProvider(DogecoinProviderBlockApi::Mainnet(api))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils;
    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    fn all_providers() -> Vec<BlockApi> {
        BitcoinMainnetExplorerBlockApi::iter()
            .map(BitcoinMainnetProviderBlockApi::Mainnet)
            .map(BlockApi::BitcoinMainnetProvider)
            .chain(
                BitcoinTestnetExplorerBlockApi::iter()
                    .map(BitcoinTestnetProviderBlockApi::Testnet)
                    .map(BlockApi::BitcoinTestnetProvider),
            )
            .chain(
                DogecoinMainnetExplorerBlockApi::iter()
                    .map(DogecoinProviderBlockApi::Mainnet)
                    .map(BlockApi::DogecoinProvider),
            )
            .collect()
    }

    /// Runs a test for the given API.
    async fn run_test(
        api: impl BlockApiTrait,
        times_called: Vec<(crate::http::HttpRequestConfig, u64)>,
        expected: serde_json::Value,
    ) {
        let response = api.fetch_data().await;
        assert_json_eq!(response, expected);

        for (config, count) in times_called {
            let request = config.request();
            assert_eq!(ic_http::mock::times_called(request), count);
        }
    }

    #[tokio::test]
    async fn test_http_request_abusing_api() {
        test_utils::mock_all_outcalls_abusing_api();
        for provider in all_providers() {
            let response = provider.fetch_data().await;

            assert_eq!(response, json!({}), "provider: {:?}", provider);
        }
    }

    #[tokio::test]
    async fn test_http_request_failed_with_404() {
        test_utils::mock_all_outcalls_404();
        for provider in all_providers() {
            let response = provider.fetch_data().await;

            assert_eq!(response, json!({}), "provider: {:?}", provider);
        }
    }

    #[test]
    fn test_names() {
        let expected: std::collections::HashMap<BlockApi, &str> = [
            (
                BitcoinMainnetExplorerBlockApi::ApiBitapsCom.into(),
                "bitcoin_api_bitaps_com_mainnet",
            ),
            (
                BitcoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                "bitcoin_api_blockchair_com_mainnet",
            ),
            (
                BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                "bitcoin_api_blockcypher_com_mainnet",
            ),
            (
                BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                "bitcoin_blockchain_info_mainnet",
            ),
            (
                BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                "bitcoin_blockstream_info_mainnet",
            ),
            (
                BitcoinMainnetProviderBlockApi::BitcoinCanister.into(),
                "bitcoin_canister",
            ),
            (
                BitcoinMainnetExplorerBlockApi::Mempool.into(),
                "bitcoin_mempool_mainnet",
            ),
            (
                BitcoinTestnetExplorerBlockApi::Mempool.into(),
                "bitcoin_mempool_testnet",
            ),
            (
                DogecoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                "dogecoin_api_blockchair_com_mainnet",
            ),
            (
                DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                "dogecoin_api_blockcypher_com_mainnet",
            ),
            (
                DogecoinProviderBlockApi::DogecoinCanister.into(),
                "dogecoin_canister",
            ),
            (
                DogecoinMainnetExplorerBlockApi::TokenView.into(),
                "dogecoin_tokenview_mainnet",
            ),
        ]
        .iter()
        .cloned()
        .collect();
        for provider in all_providers() {
            assert_eq!(provider.to_string(), expected[&provider].to_string());
        }
    }

    mod bitcoin_provider_block_api {
        use super::*;

        #[tokio::test]
        async fn test_api_bitaps_com_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBitapsCom,
                ),
                vec![(endpoint_api_bitaps_com_block_mainnet(), 1)],
                json!({
                    "height": 700001,
                    "hash": "0000000000000000000aaa111111111111111111111111111111111111111111",
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_api_blockchair_com_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBlockchairCom,
                ),
                vec![(endpoint_api_blockchair_com_block_mainnet(), 1)],
                json!({
                    "height": 700002,
                    "hash": "0000000000000000000aaa222222222222222222222222222222222222222222",
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_api_blockcypher_com_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom,
                ),
                vec![(endpoint_api_blockcypher_com_block_mainnet(), 1)],
                json!({
                "height": 700003,
                "hash": "0000000000000000000aaa333333333333333333333333333333333333333333",
                "previous_hash": "0000000000000000000aaa222222222222222222222222222222222222222222",
            }),
            )
                .await;
        }

        #[tokio::test]
        async fn test_bitcoin_canister_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::BitcoinCanister,
                vec![(endpoint_bitcoin_canister(), 1)],
                json!({
                    "height": 700007,
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_blockchain_info_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockchainInfo,
                ),
                vec![
                    (endpoint_blockchain_info_hash_mainnet(), 1),
                    (endpoint_blockchain_info_height_mainnet(), 1),
                ],
                json!({
                    "height": 700004,
                    "hash": "0000000000000000000aaa444444444444444444444444444444444444444444",
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_blockstream_info_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockstreamInfo,
                ),
                vec![
                    (endpoint_blockstream_info_hash_mainnet(), 1),
                    (endpoint_blockstream_info_height_mainnet(), 1),
                ],
                json!({
                    "height": 700005,
                    "hash": "0000000000000000000aaa555555555555555555555555555555555555555555",
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_mempool_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::Mainnet(BitcoinMainnetExplorerBlockApi::Mempool),
                vec![(endpoint_mempool_height_mainnet(), 1)],
                json!({
                    "height": 700008,
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_mempool_testnet() {
            test_utils::mock_bitcoin_testnet_outcalls();
            run_test(
                BitcoinTestnetProviderBlockApi::Testnet(BitcoinTestnetExplorerBlockApi::Mempool),
                vec![(endpoint_mempool_height_testnet(), 1)],
                json!({
                    "height": 55002,
                }),
            )
            .await;
        }
    }

    mod dogecoin_provider_block_api {
        use super::*;

        #[tokio::test]
        async fn test_api_blockchair_com_mainnet() {
            test_utils::mock_dogecoin_mainnet_outcalls();
            run_test(
                DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::ApiBlockchairCom,
                ),
                vec![(endpoint_dogecoin_api_blockchair_com_block_mainnet(), 1)],
                json!({
                    "height": 5926987,
                    "hash": "36134366860560c09a6b216cdb6ef58e4ef73792fba514e6e04d074382d0974c",
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_api_blockcypher_com_mainnet() {
            test_utils::mock_dogecoin_mainnet_outcalls();
            run_test(
                DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom,
                ),
                vec![(endpoint_dogecoin_api_blockcypher_com_block_mainnet(), 1)],
                json!({
                "height": 5926989,
                "hash": "bfbcae1f6dcc41710caad2f638dbe9b4006f6c4dd456b99a12253b4152e55cf6",
                "previous_hash": "0037287a6dfa3426da3e644da91d00b2d240a829b9b2a30d256b7eef89b78068",
            }),
            )
                .await;
        }

        #[tokio::test]
        async fn test_tokenview_mainnet() {
            test_utils::mock_dogecoin_mainnet_outcalls();
            run_test(
                DogecoinProviderBlockApi::Mainnet(DogecoinMainnetExplorerBlockApi::TokenView),
                vec![(endpoint_dogecoin_tokenview_height_mainnet(), 1)],
                json!({
                    "height": 5931072,
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_dogecoin_canister_mainnet() {
            test_utils::mock_dogecoin_mainnet_outcalls();
            run_test(
                DogecoinProviderBlockApi::DogecoinCanister,
                vec![(endpoint_dogecoin_canister(), 1)],
                json!({
                    "height": 5931098,
                }),
            )
            .await;
        }
    }
}
