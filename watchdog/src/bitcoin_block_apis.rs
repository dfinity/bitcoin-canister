use crate::endpoints::*;
use serde_json::json;

/// APIs that serve Bitcoin block data.
#[derive(Debug)]
pub enum BitcoinBlockApi {
    // TODO: investigate why this API is not working.
    #[allow(dead_code)]
    ApiBitapsCom,

    ApiBlockchairCom,
    ApiBlockcypherCom,
    BitcoinCanister,
    BlockchainInfo,
    BlockstreamInfo,

    // TODO: investigate why this API is not working.
    #[allow(dead_code)]
    ChainApiBtcCom,
}

impl BitcoinBlockApi {
    /// Fetches the block data from the API.
    pub async fn fetch_data(&self) -> serde_json::Value {
        match self {
            BitcoinBlockApi::ApiBitapsCom => http_request(endpoint_api_bitaps_com_block()).await,
            BitcoinBlockApi::ApiBlockchairCom => {
                http_request(endpoint_api_blockchair_com_block()).await
            }
            BitcoinBlockApi::ApiBlockcypherCom => {
                http_request(endpoint_api_blockcypher_com_block()).await
            }
            BitcoinBlockApi::BitcoinCanister => http_request(endpoint_bitcoin_canister()).await,
            BitcoinBlockApi::BlockchainInfo => {
                let futures = vec![
                    http_request(endpoint_blockchain_info_height()),
                    http_request(endpoint_blockchain_info_hash()),
                ];
                let results = futures::future::join_all(futures).await;
                json!({
                    "height": results[0]["height"],
                    "hash": results[1]["hash"],
                })
            }
            BitcoinBlockApi::BlockstreamInfo => {
                let futures = vec![
                    http_request(endpoint_blockstream_info_height()),
                    http_request(endpoint_blockstream_info_hash()),
                ];
                let results = futures::future::join_all(futures).await;
                json!({
                    "height": results[0]["height"],
                    "hash": results[1]["hash"],
                })
            }
            BitcoinBlockApi::ChainApiBtcCom => {
                http_request(endpoint_chain_api_btc_com_block()).await
            }
        }
    }
}

/// Makes an HTTP request to the given endpoint and returns the response as a JSON value.
async fn http_request(config: crate::http::HttpRequestConfig) -> serde_json::Value {
    let request = config.request();
    let (response,) = ic_http::http_request(request).await.unwrap();
    let json_str = String::from_utf8(response.body).expect("Raw response is not UTF-8 encoded.");
    serde_json::from_str(&json_str).expect("Failed to parse JSON from string")
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
        test_utils::mock_all_outcalls();

        let response = api.fetch_data().await;
        assert_json_eq!(response, expected);

        for (config, count) in times_called {
            let request = config.request();
            assert_eq!(ic_http::mock::times_called(request), count);
        }
    }

    #[tokio::test]
    async fn test_api_bitaps_com() {
        run_test(
            BitcoinBlockApi::ApiBitapsCom,
            vec![(endpoint_api_bitaps_com_block(), 1)],
            json!({
                "height": 700001,
                "hash": "0000000000000000000aaa111111111111111111111111111111111111111111",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_api_blockchair_com() {
        run_test(
            BitcoinBlockApi::ApiBlockchairCom,
            vec![(endpoint_api_blockchair_com_block(), 1)],
            json!({
                "height": 700002,
                "hash": "0000000000000000000aaa222222222222222222222222222222222222222222",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_api_blockcypher_com() {
        run_test(
            BitcoinBlockApi::ApiBlockcypherCom,
            vec![(endpoint_api_blockcypher_com_block(), 1)],
            json!({
                "height": 700003,
                "hash": "0000000000000000000aaa333333333333333333333333333333333333333333",
                "previous_hash": "0000000000000000000aaa222222222222222222222222222222222222222222",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_bitcoin_canister() {
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
    async fn test_blockchain_info() {
        run_test(
            BitcoinBlockApi::BlockchainInfo,
            vec![
                (endpoint_blockchain_info_hash(), 1),
                (endpoint_blockchain_info_height(), 1),
            ],
            json!({
                "height": 700004,
                "hash": "0000000000000000000aaa444444444444444444444444444444444444444444",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_blockstream_info() {
        run_test(
            BitcoinBlockApi::BlockstreamInfo,
            vec![
                (endpoint_blockstream_info_hash(), 1),
                (endpoint_blockstream_info_height(), 1),
            ],
            json!({
                "height": 700005,
                "hash": "0000000000000000000aaa555555555555555555555555555555555555555555",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_chain_api_btc_com() {
        run_test(
            BitcoinBlockApi::ChainApiBtcCom,
            vec![(endpoint_chain_api_btc_com_block(), 1)],
            json!({
                "height": 700006,
                "hash": "0000000000000000000aaa666666666666666666666666666666666666666666",
                "previous_hash": "0000000000000000000aaa555555555555555555555555555555555555555555",
            }),
        )
        .await;
    }
}
