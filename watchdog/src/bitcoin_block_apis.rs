use crate::config::BitcoinNetwork;
use crate::endpoints::*;
use crate::print;
use candid::CandidType;
use ic_cdk::api::management_canister::http_request::HttpResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;

/// APIs that serve Bitcoin block data.
#[derive(Debug, Clone, Eq, PartialEq, Hash, CandidType, Serialize, Deserialize)]
pub enum BitcoinBlockApi {
    #[serde(rename = "api_bitaps_com_mainnet")]
    ApiBitapsComMainnet,

    #[serde(rename = "api_bitaps_com_testnet")]
    ApiBitapsComTestnet,

    #[serde(rename = "api_blockchair_com_mainnet")]
    ApiBlockchairComMainnet,

    #[serde(rename = "api_blockchair_com_testnet")]
    ApiBlockchairComTestnet,

    #[serde(rename = "api_blockcypher_com_mainnet")]
    ApiBlockcypherComMainnet,

    #[serde(rename = "api_blockcypher_com_testnet")]
    ApiBlockcypherComTestnet,

    #[serde(rename = "bitcoin_canister")]
    BitcoinCanister, // Not an explorer.

    #[serde(rename = "blockchain_info_mainnet")]
    BlockchainInfoMainnet,

    #[serde(rename = "blockstream_info_mainnet")]
    BlockstreamInfoMainnet,

    #[serde(rename = "blockstream_info_testnet")]
    BlockstreamInfoTestnet,

    #[serde(rename = "chain_api_btc_com_mainnet")]
    ChainApiBtcComMainnet,
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

impl BitcoinBlockApi {
    /// Returns the list of all API providers.
    pub fn network_providers(bitcoin_network: BitcoinNetwork) -> Vec<Self> {
        match bitcoin_network {
            BitcoinNetwork::Mainnet => Self::providers_mainnet(),
            BitcoinNetwork::Testnet => Self::providers_testnet(),
        }
    }

    /// Returns the list of explorers only.
    pub fn network_explorers(bitcoin_network: BitcoinNetwork) -> Vec<Self> {
        match bitcoin_network {
            BitcoinNetwork::Mainnet => Self::explorers_mainnet(),
            BitcoinNetwork::Testnet => Self::explorers_testnet(),
        }
    }

    /// Returns the list of all mainnet API providers.
    fn providers_mainnet() -> Vec<Self> {
        let mut providers = Self::explorers_mainnet();
        // Add the Bitcoin canister, since it's not an explorer.
        providers.push(BitcoinBlockApi::BitcoinCanister);

        providers
    }

    /// Returns the list of all testnet API providers.
    fn providers_testnet() -> Vec<Self> {
        let mut providers = Self::explorers_testnet();
        // Add the Bitcoin canister, since it's not an explorer.
        providers.push(BitcoinBlockApi::BitcoinCanister);

        providers
    }

    /// Returns the list of mainnet explorers only.
    fn explorers_mainnet() -> Vec<Self> {
        let mut explorers = vec![
            BitcoinBlockApi::ApiBitapsComMainnet,
            BitcoinBlockApi::ApiBlockchairComMainnet,
            BitcoinBlockApi::ApiBlockcypherComMainnet,
            BitcoinBlockApi::BlockchainInfoMainnet,
            BitcoinBlockApi::BlockstreamInfoMainnet,
            BitcoinBlockApi::ChainApiBtcComMainnet,
        ];
        // Remove the explorers that are not configured.
        let configured: HashSet<_> = crate::storage::get_config().explorers.into_iter().collect();
        explorers.retain(|x| configured.contains(x));

        explorers
    }

    /// Returns the list of testnet explorers only.
    fn explorers_testnet() -> Vec<Self> {
        let mut explorers = vec![
            BitcoinBlockApi::ApiBitapsComTestnet,
            BitcoinBlockApi::ApiBlockchairComTestnet,
            BitcoinBlockApi::ApiBlockcypherComTestnet,
            BitcoinBlockApi::BlockstreamInfoTestnet,
        ];
        // Remove the explorers that are not configured.
        let configured: HashSet<_> = crate::storage::get_config().explorers.into_iter().collect();
        explorers.retain(|x| configured.contains(x));

        explorers
    }

    /// Fetches the block data from the API.
    pub async fn fetch_data(&self) -> serde_json::Value {
        match self {
            BitcoinBlockApi::ApiBitapsComMainnet => {
                http_request(endpoint_api_bitaps_com_block_mainnet()).await
            }
            BitcoinBlockApi::ApiBitapsComTestnet => {
                http_request(endpoint_api_bitaps_com_block_testnet()).await
            }
            BitcoinBlockApi::ApiBlockchairComMainnet => {
                http_request(endpoint_api_blockchair_com_block_mainnet()).await
            }
            BitcoinBlockApi::ApiBlockchairComTestnet => {
                http_request(endpoint_api_blockchair_com_block_testnet()).await
            }
            BitcoinBlockApi::ApiBlockcypherComMainnet => {
                http_request(endpoint_api_blockcypher_com_block_mainnet()).await
            }
            BitcoinBlockApi::ApiBlockcypherComTestnet => {
                http_request(endpoint_api_blockcypher_com_block_testnet()).await
            }
            BitcoinBlockApi::BitcoinCanister => http_request(endpoint_bitcoin_canister()).await,
            BitcoinBlockApi::BlockchainInfoMainnet => {
                let futures = vec![
                    http_request(endpoint_blockchain_info_height_mainnet()),
                    http_request(endpoint_blockchain_info_hash_mainnet()),
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
            BitcoinBlockApi::BlockstreamInfoMainnet => {
                let futures = vec![
                    http_request(endpoint_blockstream_info_height_mainnet()),
                    http_request(endpoint_blockstream_info_hash_mainnet()),
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
            BitcoinBlockApi::BlockstreamInfoTestnet => {
                let futures = vec![
                    http_request(endpoint_blockstream_info_height_testnet()),
                    http_request(endpoint_blockstream_info_hash_testnet()),
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
            BitcoinBlockApi::ChainApiBtcComMainnet => {
                http_request(endpoint_chain_api_btc_com_block_mainnet()).await
            }
        }
    }
}

/// Makes an HTTP request to the given endpoint and returns the response as a JSON value.
async fn http_request(config: crate::http::HttpRequestConfig) -> serde_json::Value {
    // Send zero cycles with the request to avoid the canister
    // to run out of cycles when deployed on a system subnet.
    let cycles = 0;
    let result = ic_http::http_request_with_cycles(config.request(), cycles).await;

    match result {
        Ok((response,)) if response.status == 200 => parse_response(response),
        Ok(_) => json!({}),
        Err(error) => {
            print(&format!("HTTP request failed: {:?}", error));
            json!({})
        }
    }
}

/// Parses the given HTTP response into a JSON value.
fn parse_response(response: HttpResponse) -> serde_json::Value {
    match String::from_utf8(response.body) {
        Ok(json_str) => serde_json::from_str(&json_str).unwrap_or_else(|error| {
            print(&format!(
                "Failed to parse JSON from string, error: {error:?}, text: {json_str:?}"
            ));
            json!({})
        }),
        Err(error) => {
            print(&format!("Raw response is not UTF-8 encoded: {:?}", error));
            json!({})
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils;
    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    /// Runs a test for the given API.
    async fn run_test(
        api: BitcoinBlockApi,
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
    async fn test_api_bitaps_com_mainnet() {
        test_utils::mock_mainnet_outcalls();
        run_test(
            BitcoinBlockApi::ApiBitapsComMainnet,
            vec![(endpoint_api_bitaps_com_block_mainnet(), 1)],
            json!({
                "height": 700001,
                "hash": "0000000000000000000aaa111111111111111111111111111111111111111111",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_api_bitaps_com_testnet() {
        test_utils::mock_testnet_outcalls();
        run_test(
            BitcoinBlockApi::ApiBitapsComTestnet,
            vec![(endpoint_api_bitaps_com_block_testnet(), 1)],
            json!({
                "height": 2000001,
                "hash": "0000000000000000000fff111111111111111111111111111111111111111111",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_api_blockchair_com_mainnet() {
        test_utils::mock_mainnet_outcalls();
        run_test(
            BitcoinBlockApi::ApiBlockchairComMainnet,
            vec![(endpoint_api_blockchair_com_block_mainnet(), 1)],
            json!({
                "height": 700002,
                "hash": "0000000000000000000aaa222222222222222222222222222222222222222222",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_api_blockchair_com_testnet() {
        test_utils::mock_testnet_outcalls();
        run_test(
            BitcoinBlockApi::ApiBlockchairComTestnet,
            vec![(endpoint_api_blockchair_com_block_testnet(), 1)],
            json!({
                "height": 2000002,
                "hash": "0000000000000000000fff222222222222222222222222222222222222222222",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_api_blockcypher_com_mainnet() {
        test_utils::mock_mainnet_outcalls();
        run_test(
            BitcoinBlockApi::ApiBlockcypherComMainnet,
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
    async fn test_api_blockcypher_com_testnet() {
        test_utils::mock_testnet_outcalls();
        run_test(
            BitcoinBlockApi::ApiBlockcypherComTestnet,
            vec![(endpoint_api_blockcypher_com_block_testnet(), 1)],
            json!({
                "height": 2000003,
                "hash": "0000000000000000000fff333333333333333333333333333333333333333333",
                "previous_hash": "0000000000000000000fff222222222222222222222222222222222222222222",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_bitcoin_canister_mainnet() {
        test_utils::mock_mainnet_outcalls();
        run_test(
            BitcoinBlockApi::BitcoinCanister,
            vec![(endpoint_bitcoin_canister(), 1)],
            json!({
                "height": 700007,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_blockchain_info_mainnet() {
        test_utils::mock_mainnet_outcalls();
        run_test(
            BitcoinBlockApi::BlockchainInfoMainnet,
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
        test_utils::mock_mainnet_outcalls();
        run_test(
            BitcoinBlockApi::BlockstreamInfoMainnet,
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
    async fn test_blockstream_info_testnet() {
        test_utils::mock_testnet_outcalls();
        run_test(
            BitcoinBlockApi::BlockstreamInfoTestnet,
            vec![
                (endpoint_blockstream_info_hash_testnet(), 1),
                (endpoint_blockstream_info_height_testnet(), 1),
            ],
            json!({
                "height": 2000004,
                "hash": "0000000000000000000fff555555555555555555555555555555555555555555",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_chain_api_btc_com_mainnet() {
        test_utils::mock_mainnet_outcalls();
        run_test(
            BitcoinBlockApi::ChainApiBtcComMainnet,
            vec![(endpoint_chain_api_btc_com_block_mainnet(), 1)],
            json!({
                "height": 700006,
                "hash": "0000000000000000000aaa666666666666666666666666666666666666666666",
                "previous_hash": "0000000000000000000aaa555555555555555555555555555555555555555555",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_http_request_failed_with_404() {
        test_utils::mock_all_outcalls_404();
        let all_providers = BitcoinBlockApi::providers_mainnet()
            .into_iter()
            .chain(BitcoinBlockApi::providers_testnet().into_iter())
            .collect::<Vec<_>>();
        for provider in all_providers {
            let response = provider.fetch_data().await;

            assert_eq!(response, json!({}), "provider: {:?}", provider);
        }
    }

    #[tokio::test]
    async fn test_http_request_abusing_api() {
        test_utils::mock_all_outcalls_abusing_api();
        for provider in BitcoinBlockApi::providers_mainnet() {
            let response = provider.fetch_data().await;

            assert_eq!(response, json!({}), "provider: {:?}", provider);
        }
    }

    #[test]
    fn test_names() {
        let expected: std::collections::HashMap<BitcoinBlockApi, &str> = [
            (
                BitcoinBlockApi::ApiBitapsComMainnet,
                "api_bitaps_com_mainnet",
            ),
            (
                BitcoinBlockApi::ApiBitapsComTestnet,
                "api_bitaps_com_testnet",
            ),
            (
                BitcoinBlockApi::ApiBlockchairComMainnet,
                "api_blockchair_com_mainnet",
            ),
            (
                BitcoinBlockApi::ApiBlockchairComTestnet,
                "api_blockchair_com_testnet",
            ),
            (
                BitcoinBlockApi::ApiBlockcypherComMainnet,
                "api_blockcypher_com_mainnet",
            ),
            (
                BitcoinBlockApi::ApiBlockcypherComTestnet,
                "api_blockcypher_com_testnet",
            ),
            (BitcoinBlockApi::BitcoinCanister, "bitcoin_canister"),
            (
                BitcoinBlockApi::BlockchainInfoMainnet,
                "blockchain_info_mainnet",
            ),
            (
                BitcoinBlockApi::BlockstreamInfoMainnet,
                "blockstream_info_mainnet",
            ),
            (
                BitcoinBlockApi::BlockstreamInfoTestnet,
                "blockstream_info_testnet",
            ),
            (
                BitcoinBlockApi::ChainApiBtcComMainnet,
                "chain_api_btc_com_mainnet",
            ),
        ]
        .iter()
        .cloned()
        .collect();
        let all_providers = BitcoinBlockApi::providers_mainnet()
            .into_iter()
            .chain(BitcoinBlockApi::providers_testnet().into_iter())
            .collect::<Vec<_>>();
        for provider in all_providers {
            assert_eq!(provider.to_string(), expected[&provider].to_string());
        }
    }
}
