use crate::config::Network;
use crate::endpoints::*;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use strum::{EnumIter, IntoEnumIterator};

/// APIs that serve block data.
#[derive(
    Clone, Debug, Eq, PartialEq, Hash, CandidType, Serialize, Deserialize, PartialOrd, Ord,
)]
pub enum CandidBlockApi {
    #[serde(rename = "api_bitaps_com_mainnet")]
    BitcoinApiBitapsComMainnet,

    #[serde(rename = "api_blockchair_com_mainnet")]
    BitcoinApiBlockchairComMainnet,

    #[serde(rename = "api_blockcypher_com_mainnet")]
    BitcoinApiBlockcypherComMainnet,

    #[serde(rename = "bitcoin_canister")]
    BitcoinCanister, // Not an explorer.

    #[serde(rename = "blockchain_info_mainnet")]
    BitcoinBlockchainInfoMainnet,

    #[serde(rename = "blockstream_info_mainnet")]
    BitcoinBlockstreamInfoMainnet,

    #[serde(rename = "mempool_mainnet")]
    BitcoinMempoolMainnet,

    #[serde(rename = "mempool_testnet")]
    BitcoinMempoolTestnet,

    #[serde(rename = "dogecoin_api_blockchair_com_mainnet")]
    DogecoinApiBlockchairComMainnet,

    #[serde(rename = "dogecoin_api_blockcypher_com_mainnet")]
    DogecoinApiBlockcypherComMainnet,

    #[serde(rename = "dogecoin_canister")]
    DogecoinCanister, // Not an explorer.

    #[serde(rename = "dogecoin_tokenview_mainnet")]
    DogecoinTokenViewMainnet,
}

impl std::fmt::Display for CandidBlockApi {
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
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum BlockApi {
    BitcoinProvider(BitcoinProviderBlockApi),
    DogecoinProvider(DogecoinProviderBlockApi),
}

impl From<BlockApi> for CandidBlockApi {
    fn from(api: BlockApi) -> Self {
        match api {
            BlockApi::BitcoinProvider(provider) => match provider {
                BitcoinProviderBlockApi::BitcoinCanister => CandidBlockApi::BitcoinCanister,
                BitcoinProviderBlockApi::Mainnet(explorer) => match explorer {
                    BitcoinMainnetExplorerBlockApi::ApiBitapsCom => {
                        CandidBlockApi::BitcoinApiBitapsComMainnet
                    }
                    BitcoinMainnetExplorerBlockApi::ApiBlockchairCom => {
                        CandidBlockApi::BitcoinApiBlockchairComMainnet
                    }
                    BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom => {
                        CandidBlockApi::BitcoinApiBlockcypherComMainnet
                    }
                    BitcoinMainnetExplorerBlockApi::BlockchainInfo => {
                        CandidBlockApi::BitcoinBlockchainInfoMainnet
                    }
                    BitcoinMainnetExplorerBlockApi::BlockexplorerOne => {
                        todo!("DEFI-2493: add BlockexplorerOne")
                    }
                    BitcoinMainnetExplorerBlockApi::BlockstreamInfo => {
                        CandidBlockApi::BitcoinBlockstreamInfoMainnet
                    }
                    BitcoinMainnetExplorerBlockApi::Mempool => {
                        CandidBlockApi::BitcoinMempoolMainnet
                    }
                },
                BitcoinProviderBlockApi::Testnet(explorer) => match explorer {
                    BitcoinTestnetExplorerBlockApi::Mempool => {
                        CandidBlockApi::BitcoinMempoolTestnet
                    }
                },
            },
            BlockApi::DogecoinProvider(provider) => match provider {
                DogecoinProviderBlockApi::DogecoinCanister => CandidBlockApi::DogecoinCanister,
                DogecoinProviderBlockApi::Mainnet(explorer) => match explorer {
                    DogecoinMainnetExplorerBlockApi::ApiBlockchairCom => {
                        CandidBlockApi::DogecoinApiBlockchairComMainnet
                    }
                    DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom => {
                        CandidBlockApi::DogecoinApiBlockcypherComMainnet
                    }
                    DogecoinMainnetExplorerBlockApi::TokenView => {
                        CandidBlockApi::DogecoinTokenViewMainnet
                    }
                },
            },
        }
    }
}

/// Providers that serve Bitcoin block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BitcoinProviderBlockApi {
    BitcoinCanister,
    Mainnet(BitcoinMainnetExplorerBlockApi),
    Testnet(BitcoinTestnetExplorerBlockApi),
}

impl From<BitcoinProviderBlockApi> for CandidBlockApi {
    fn from(api: BitcoinProviderBlockApi) -> Self {
        CandidBlockApi::from(BlockApi::BitcoinProvider(api))
    }
}

/// Explorers that serve Bitcoin mainnet block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, EnumIter)]
pub enum BitcoinMainnetExplorerBlockApi {
    ApiBitapsCom,
    ApiBlockchairCom,
    ApiBlockcypherCom,
    BlockchainInfo,
    BlockexplorerOne,
    BlockstreamInfo,
    Mempool,
}

/// Explorers that serve Bitcoin testnet block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, EnumIter)]
pub enum BitcoinTestnetExplorerBlockApi {
    Mempool,
}

/// Providers that serve Dogecoin block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DogecoinProviderBlockApi {
    DogecoinCanister,
    Mainnet(DogecoinMainnetExplorerBlockApi),
}

impl From<DogecoinProviderBlockApi> for CandidBlockApi {
    fn from(api: DogecoinProviderBlockApi) -> Self {
        CandidBlockApi::from(BlockApi::DogecoinProvider(api))
    }
}

/// Explorers that serve Dogecoin mainnet block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, EnumIter)]
pub enum DogecoinMainnetExplorerBlockApi {
    ApiBlockchairCom,
    ApiBlockcypherCom,
    TokenView,
}

impl From<BitcoinMainnetExplorerBlockApi> for BlockApi {
    fn from(api: BitcoinMainnetExplorerBlockApi) -> Self {
        BlockApi::BitcoinProvider(BitcoinProviderBlockApi::Mainnet(api))
    }
}

impl From<BitcoinTestnetExplorerBlockApi> for BlockApi {
    fn from(api: BitcoinTestnetExplorerBlockApi) -> Self {
        BlockApi::BitcoinProvider(BitcoinProviderBlockApi::Testnet(api))
    }
}

impl From<DogecoinMainnetExplorerBlockApi> for BlockApi {
    fn from(api: DogecoinMainnetExplorerBlockApi) -> Self {
        BlockApi::DogecoinProvider(DogecoinProviderBlockApi::Mainnet(api))
    }
}

impl From<BitcoinMainnetExplorerBlockApi> for CandidBlockApi {
    fn from(api: BitcoinMainnetExplorerBlockApi) -> Self {
        CandidBlockApi::from(BlockApi::from(api))
    }
}

impl From<BitcoinTestnetExplorerBlockApi> for CandidBlockApi {
    fn from(api: BitcoinTestnetExplorerBlockApi) -> Self {
        CandidBlockApi::from(BlockApi::from(api))
    }
}

impl From<DogecoinMainnetExplorerBlockApi> for CandidBlockApi {
    fn from(api: DogecoinMainnetExplorerBlockApi) -> Self {
        CandidBlockApi::from(BlockApi::from(api))
    }
}

impl BlockApi {
    /// Fetches the block data from the API.
    pub async fn fetch_data(&self) -> serde_json::Value {
        match self {
            Self::BitcoinProvider(BitcoinProviderBlockApi::BitcoinCanister) => {
                endpoint_bitcoin_canister().send_request_json().await
            }
            Self::DogecoinProvider(DogecoinProviderBlockApi::DogecoinCanister) => {
                endpoint_dogecoin_canister().send_request_json().await
            }
            Self::BitcoinProvider(BitcoinProviderBlockApi::Mainnet(api)) => match api {
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
                BitcoinMainnetExplorerBlockApi::BlockexplorerOne => {
                    endpoint_blockexplorer_one_block_mainnet()
                        .send_request_json()
                        .await
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
            Self::BitcoinProvider(BitcoinProviderBlockApi::Testnet(api)) => match api {
                BitcoinTestnetExplorerBlockApi::Mempool => {
                    endpoint_mempool_height_testnet().send_request_json().await
                }
            },
            Self::DogecoinProvider(DogecoinProviderBlockApi::Mainnet(api)) => match api {
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

    /// Returns the canister API for the given network.
    pub fn network_canister(network: Network) -> Self {
        match network {
            Network::BitcoinMainnet | Network::BitcoinTestnet => {
                Self::BitcoinProvider(BitcoinProviderBlockApi::BitcoinCanister)
            }
            Network::DogecoinMainnet => {
                Self::DogecoinProvider(DogecoinProviderBlockApi::DogecoinCanister)
            }
        }
    }

    /// Returns the list of all API providers.
    pub fn network_providers(network: Network) -> Vec<Self> {
        match network {
            Network::BitcoinMainnet => BitcoinProviderBlockApi::providers_mainnet()
                .iter()
                .map(|&x| Self::BitcoinProvider(x))
                .collect(),
            Network::BitcoinTestnet => BitcoinProviderBlockApi::providers_testnet()
                .iter()
                .map(|&x| Self::BitcoinProvider(x))
                .collect(),
            Network::DogecoinMainnet => DogecoinProviderBlockApi::providers_mainnet()
                .iter()
                .map(|&x| Self::DogecoinProvider(x))
                .collect(),
        }
    }

    /// Returns the list of explorers only.
    pub fn network_explorers(network: Network) -> Vec<Self> {
        match network {
            Network::BitcoinMainnet => BitcoinProviderBlockApi::explorers_mainnet()
                .iter()
                .map(|&x| Self::BitcoinProvider(x))
                .collect(),
            Network::BitcoinTestnet => BitcoinProviderBlockApi::explorers_testnet()
                .iter()
                .map(|&x| Self::BitcoinProvider(x))
                .collect(),
            Network::DogecoinMainnet => DogecoinProviderBlockApi::explorers_mainnet()
                .iter()
                .map(|&x| Self::DogecoinProvider(x))
                .collect(),
        }
    }
}

impl BitcoinProviderBlockApi {
    /// Returns the list of all Bitcoin mainnet API providers.
    fn providers_mainnet() -> Vec<Self> {
        let mut providers: Vec<BitcoinProviderBlockApi> = Self::explorers_mainnet();
        // Add the Bitcoin canister, since it's not an explorer.
        providers.push(BitcoinProviderBlockApi::BitcoinCanister);

        providers
    }

    /// Returns the list of all Bitcoin testnet API providers.
    fn providers_testnet() -> Vec<Self> {
        let mut providers = Self::explorers_testnet();
        // Add the Bitcoin canister, since it's not an explorer.
        providers.push(BitcoinProviderBlockApi::BitcoinCanister);

        providers
    }

    /// Returns the list of Bitcoin mainnet explorers only.
    fn explorers_mainnet() -> Vec<Self> {
        let mut explorers: Vec<BitcoinProviderBlockApi> = BitcoinMainnetExplorerBlockApi::iter()
            .filter(|explorer| *explorer != BitcoinMainnetExplorerBlockApi::BlockexplorerOne) // TODO(DEFI-2493): add BlockexplorerOne
            .map(BitcoinProviderBlockApi::Mainnet)
            .collect();
        // Remove the explorers that are not configured.
        let configured: BTreeSet<_> = crate::storage::get_config().explorers.into_iter().collect();
        explorers.retain(|&x| configured.contains(&BlockApi::BitcoinProvider(x).into()));

        explorers
    }

    /// Returns the list of Bitcoin testnet explorers only.
    fn explorers_testnet() -> Vec<Self> {
        let mut explorers: Vec<BitcoinProviderBlockApi> = BitcoinTestnetExplorerBlockApi::iter()
            .map(BitcoinProviderBlockApi::Testnet)
            .collect();
        // Remove the explorers that are not configured.
        let configured: BTreeSet<_> = crate::storage::get_config().explorers.into_iter().collect();
        explorers.retain(|&x| configured.contains(&BlockApi::BitcoinProvider(x).into()));

        explorers
    }
}

impl DogecoinProviderBlockApi {
    /// Returns the list of all Dogecoin mainnet API providers.
    fn providers_mainnet() -> Vec<Self> {
        let mut providers: Vec<DogecoinProviderBlockApi> = Self::explorers_mainnet();
        // Add the Dogecoin canister, since it's not an explorer.
        providers.push(DogecoinProviderBlockApi::DogecoinCanister);

        providers
    }

    /// Returns the list of Dogecoin mainnet explorers only.
    fn explorers_mainnet() -> Vec<Self> {
        let mut explorers: Vec<DogecoinProviderBlockApi> = DogecoinMainnetExplorerBlockApi::iter()
            .map(DogecoinProviderBlockApi::Mainnet)
            .collect();
        // Remove the explorers that are not configured.
        let configured: BTreeSet<_> = crate::storage::get_config().explorers.into_iter().collect();
        explorers.retain(|&x| configured.contains(&BlockApi::DogecoinProvider(x).into()));

        explorers
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
            .filter(|explorer| *explorer != BitcoinMainnetExplorerBlockApi::BlockexplorerOne) // TODO(DEFI-2493): add BlockexplorerOne
            .map(BitcoinProviderBlockApi::Mainnet)
            .map(BlockApi::BitcoinProvider)
            .chain(
                BitcoinTestnetExplorerBlockApi::iter()
                    .map(BitcoinProviderBlockApi::Testnet)
                    .map(BlockApi::BitcoinProvider),
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
        api: BlockApi,
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
        let expected: std::collections::HashMap<CandidBlockApi, &str> = [
            (
                BitcoinMainnetExplorerBlockApi::ApiBitapsCom.into(),
                "api_bitaps_com_mainnet",
            ),
            (
                BitcoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                "api_blockchair_com_mainnet",
            ),
            (
                BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                "api_blockcypher_com_mainnet",
            ),
            (
                BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                "blockchain_info_mainnet",
            ),
            // (
            //     BitcoinMainnetExplorerBlockApi::BlockexplorerOne.into(),
            //     "blockexplorer_one_mainnet",
            // ), // TODO(DEFI-2493): add BlockexplorerOne
            (
                BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                "blockstream_info_mainnet",
            ),
            (
                BitcoinProviderBlockApi::BitcoinCanister.into(),
                "bitcoin_canister",
            ),
            (
                BitcoinMainnetExplorerBlockApi::Mempool.into(),
                "mempool_mainnet",
            ),
            (
                BitcoinTestnetExplorerBlockApi::Mempool.into(),
                "mempool_testnet",
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
        let all_candid_providers = all_providers()
            .into_iter()
            .map(Into::into)
            .collect::<Vec<CandidBlockApi>>();
        for provider in all_candid_providers {
            assert_eq!(provider.to_string(), expected[&provider].to_string());
        }
    }

    mod bitcoin_provider_block_api {
        use super::*;

        #[tokio::test]
        async fn test_api_bitaps_com_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BlockApi::BitcoinProvider(BitcoinProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBitapsCom,
                )),
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
                BlockApi::BitcoinProvider(BitcoinProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBlockchairCom,
                )),
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
                BlockApi::BitcoinProvider(BitcoinProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom,
                )),
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
                BlockApi::BitcoinProvider(BitcoinProviderBlockApi::BitcoinCanister),
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
                BlockApi::BitcoinProvider(BitcoinProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockchainInfo,
                )),
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
        async fn test_blockexplorer_one_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BlockApi::BitcoinProvider(BitcoinProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockexplorerOne,
                )),
                vec![(endpoint_blockexplorer_one_block_mainnet(), 1)],
                json!({
                    "height": 923450,
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_blockstream_info_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BlockApi::BitcoinProvider(BitcoinProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::BlockstreamInfo,
                )),
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
                BlockApi::BitcoinProvider(BitcoinProviderBlockApi::Mainnet(
                    BitcoinMainnetExplorerBlockApi::Mempool,
                )),
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
                BlockApi::BitcoinProvider(BitcoinProviderBlockApi::Testnet(
                    BitcoinTestnetExplorerBlockApi::Mempool,
                )),
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
                BlockApi::DogecoinProvider(DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::ApiBlockchairCom,
                )),
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
                BlockApi::DogecoinProvider(DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom,
                )),
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
                BlockApi::DogecoinProvider(DogecoinProviderBlockApi::Mainnet(
                    DogecoinMainnetExplorerBlockApi::TokenView,
                )),
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
                BlockApi::DogecoinProvider(DogecoinProviderBlockApi::DogecoinCanister),
                vec![(endpoint_dogecoin_canister(), 1)],
                json!({
                    "height": 5931098,
                }),
            )
            .await;
        }
    }
}
