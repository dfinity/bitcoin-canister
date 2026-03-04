use crate::endpoints::*;
use async_trait::async_trait;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::{Display, EnumIter, EnumString};

#[async_trait]
pub trait BlockProvider {
    async fn fetch_data(&self) -> serde_json::Value;
    fn name(&self) -> String;
}

/// APIs that serve Bitcoin block data, for legacy purpose.
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

/// Providers that serve Bitcoin mainnet block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Display, EnumString, EnumIter)]
pub enum BitcoinMainnetProviderBlockApi {
    #[strum(serialize = "bitcoin_canister")]
    BitcoinCanister,
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

#[async_trait]
impl BlockProvider for BitcoinMainnetProviderBlockApi {
    async fn fetch_data(&self) -> serde_json::Value {
        match self {
            Self::BitcoinCanister => endpoint_bitcoin_canister().send_request_json().await,
            Self::ApiBitapsCom => {
                endpoint_bitcoin_mainnet_api_bitaps_com()
                    .send_request_json()
                    .await
            }
            Self::ApiBlockchairCom => {
                endpoint_bitcoin_mainnet_api_blockchair_com()
                    .send_request_json()
                    .await
            }
            Self::ApiBlockcypherCom => {
                endpoint_bitcoin_mainnet_api_blockcypher_com()
                    .send_request_json()
                    .await
            }
            Self::BlockchainInfo => {
                endpoint_bitcoin_mainnet_blockchain_info()
                    .send_request_json()
                    .await
            }
            Self::BlockstreamInfo => {
                endpoint_bitcoin_mainnet_blockstream_info()
                    .send_request_json()
                    .await
            }
            Self::Mempool => endpoint_bitcoin_mainnet_mempool().send_request_json().await,
        }
    }

    fn name(&self) -> String {
        self.to_string()
    }
}

/// Providers that serve testnet Bitcoin block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Display, EnumString, EnumIter)]
pub enum BitcoinTestnetProviderBlockApi {
    #[strum(serialize = "bitcoin_canister")]
    BitcoinCanister,
    #[strum(serialize = "bitcoin_mempool_testnet")]
    Mempool,
}

#[async_trait]
impl BlockProvider for BitcoinTestnetProviderBlockApi {
    async fn fetch_data(&self) -> serde_json::Value {
        match self {
            Self::BitcoinCanister => endpoint_bitcoin_canister().send_request_json().await,
            Self::Mempool => endpoint_bitcoin_testnet_mempool().send_request_json().await,
        }
    }

    fn name(&self) -> String {
        self.to_string()
    }
}

/// Providers that serve Dogecoin block data.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Display, EnumString, EnumIter)]
pub enum DogecoinProviderBlockApi {
    #[strum(serialize = "dogecoin_canister")]
    DogecoinCanister,
    #[strum(serialize = "dogecoin_api_blockchair_com_mainnet")]
    ApiBlockchairCom,
    #[strum(serialize = "dogecoin_api_blockcypher_com_mainnet")]
    ApiBlockcypherCom,
    #[strum(serialize = "dogecoin_psy_protocol_mainnet")]
    PsyProtocol,
}

#[async_trait]
impl BlockProvider for DogecoinProviderBlockApi {
    async fn fetch_data(&self) -> serde_json::Value {
        match self {
            Self::DogecoinCanister => endpoint_dogecoin_canister().send_request_json().await,
            Self::ApiBlockchairCom => {
                endpoint_dogecoin_mainnet_api_blockchair_com()
                    .send_request_json()
                    .await
            }
            Self::ApiBlockcypherCom => {
                endpoint_dogecoin_mainnet_api_blockcypher_com()
                    .send_request_json()
                    .await
            }
            Self::PsyProtocol => {
                endpoint_dogecoin_mainnet_psy_protocol()
                    .send_request_json()
                    .await
            }
        }
    }

    fn name(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils;
    use assert_json_diff::assert_json_eq;
    use serde_json::json;
    use strum::IntoEnumIterator;

    #[tokio::test]
    async fn test_http_request_abusing_api() {
        test_utils::mock_all_outcalls_abusing_api();

        for provider in BitcoinMainnetProviderBlockApi::iter() {
            let response = provider.fetch_data().await;
            assert_eq!(response, json!({}), "provider: {:?}", provider);
        }

        for provider in BitcoinTestnetProviderBlockApi::iter() {
            let response = provider.fetch_data().await;
            assert_eq!(response, json!({}), "provider: {:?}", provider);
        }

        for provider in DogecoinProviderBlockApi::iter() {
            let response = provider.fetch_data().await;
            assert_eq!(response, json!({}), "provider: {:?}", provider);
        }
    }

    #[tokio::test]
    async fn test_http_request_failed_with_404() {
        test_utils::mock_all_outcalls_404();

        for provider in BitcoinMainnetProviderBlockApi::iter() {
            let response = provider.fetch_data().await;
            assert_eq!(response, json!({}), "provider: {:?}", provider);
        }

        for provider in BitcoinTestnetProviderBlockApi::iter() {
            let response = provider.fetch_data().await;
            assert_eq!(response, json!({}), "provider: {:?}", provider);
        }

        for provider in DogecoinProviderBlockApi::iter() {
            let response = provider.fetch_data().await;
            assert_eq!(response, json!({}), "provider: {:?}", provider);
        }
    }

    #[test]
    fn test_names() {
        assert_eq!(
            BitcoinMainnetProviderBlockApi::ApiBitapsCom.to_string(),
            "bitcoin_api_bitaps_com_mainnet"
        );
        assert_eq!(
            BitcoinMainnetProviderBlockApi::ApiBlockchairCom.to_string(),
            "bitcoin_api_blockchair_com_mainnet"
        );
        assert_eq!(
            BitcoinMainnetProviderBlockApi::ApiBlockcypherCom.to_string(),
            "bitcoin_api_blockcypher_com_mainnet"
        );
        assert_eq!(
            BitcoinMainnetProviderBlockApi::BlockchainInfo.to_string(),
            "bitcoin_blockchain_info_mainnet"
        );
        assert_eq!(
            BitcoinMainnetProviderBlockApi::BlockstreamInfo.to_string(),
            "bitcoin_blockstream_info_mainnet"
        );
        assert_eq!(
            BitcoinMainnetProviderBlockApi::Mempool.to_string(),
            "bitcoin_mempool_mainnet"
        );

        assert_eq!(
            BitcoinMainnetProviderBlockApi::BitcoinCanister.to_string(),
            "bitcoin_canister"
        );

        assert_eq!(
            BitcoinTestnetProviderBlockApi::Mempool.to_string(),
            "bitcoin_mempool_testnet"
        );

        assert_eq!(
            DogecoinProviderBlockApi::ApiBlockchairCom.to_string(),
            "dogecoin_api_blockchair_com_mainnet"
        );
        assert_eq!(
            DogecoinProviderBlockApi::ApiBlockcypherCom.to_string(),
            "dogecoin_api_blockcypher_com_mainnet"
        );
        assert_eq!(
            DogecoinProviderBlockApi::PsyProtocol.to_string(),
            "dogecoin_psy_protocol_mainnet"
        );

        assert_eq!(
            DogecoinProviderBlockApi::DogecoinCanister.to_string(),
            "dogecoin_canister"
        );
    }

    /// Runs a test for the given API.
    async fn run_test(
        api: impl BlockProvider,
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

    mod bitcoin_provider_block_api {
        use super::*;

        #[tokio::test]
        async fn test_api_bitaps_com_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::ApiBitapsCom,
                vec![(endpoint_bitcoin_mainnet_api_bitaps_com(), 1)],
                json!({
                    "height": 700001,
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_api_blockchair_com_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::ApiBlockchairCom,
                vec![(endpoint_bitcoin_mainnet_api_blockchair_com(), 1)],
                json!({
                    "height": 700002,
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_api_blockcypher_com_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::ApiBlockcypherCom,
                vec![(endpoint_bitcoin_mainnet_api_blockcypher_com(), 1)],
                json!({
                    "height": 700003,
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
                BitcoinMainnetProviderBlockApi::BlockchainInfo,
                vec![(endpoint_bitcoin_mainnet_blockchain_info(), 1)],
                json!({
                    "height": 700004,
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_blockstream_info_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::BlockstreamInfo,
                vec![(endpoint_bitcoin_mainnet_blockstream_info(), 1)],
                json!({
                    "height": 700005,
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_mempool_mainnet() {
            test_utils::mock_bitcoin_mainnet_outcalls();
            run_test(
                BitcoinMainnetProviderBlockApi::Mempool,
                vec![(endpoint_bitcoin_mainnet_mempool(), 1)],
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
                BitcoinTestnetProviderBlockApi::Mempool,
                vec![(endpoint_bitcoin_testnet_mempool(), 1)],
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
                DogecoinProviderBlockApi::ApiBlockchairCom,
                vec![(endpoint_dogecoin_mainnet_api_blockchair_com(), 1)],
                json!({
                    "height": 5926987,
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_api_blockcypher_com_mainnet() {
            test_utils::mock_dogecoin_mainnet_outcalls();
            run_test(
                DogecoinProviderBlockApi::ApiBlockcypherCom,
                vec![(endpoint_dogecoin_mainnet_api_blockcypher_com(), 1)],
                json!({
                    "height": 5926989,
                }),
            )
            .await;
        }

        #[tokio::test]
        async fn test_psy_protocol_mainnet() {
            test_utils::mock_dogecoin_mainnet_outcalls();
            run_test(
                DogecoinProviderBlockApi::PsyProtocol,
                vec![(endpoint_dogecoin_mainnet_psy_protocol(), 1)],
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
