use crate::config::Network;
use crate::transform_bitcoin_mempool;
use crate::{
    http::{HttpRequestConfig, TransformFnWrapper},
    print, transform_bitcoin_mainnet_api_bitaps_com, transform_bitcoin_mainnet_api_blockchair_com,
    transform_bitcoin_mainnet_api_blockcypher_com, transform_bitcoin_mainnet_blockchain_info,
    transform_bitcoin_mainnet_blockstream_info, transform_dogecoin_mainnet_api_blockchair_com,
    transform_dogecoin_mainnet_api_blockcypher_com, transform_dogecoin_mainnet_tokenview,
};
use ic_cdk::management_canister::{HttpRequestResult, TransformArgs};
use serde_json::json;

/// Creates a config for fetching mainnet block data from api.bitaps.com.
pub fn endpoint_bitcoin_mainnet_api_bitaps_com() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://api.bitaps.com/btc/v1/blockchain/block/last",
        Some(TransformFnWrapper {
            name: "transform_bitcoin_mainnet_api_bitaps_com",
            func: transform_bitcoin_mainnet_api_bitaps_com,
        }),
        |raw| {
            apply_to_body_json(raw, |json| {
                let data = json["data"].clone();
                json!({
                    "height": data["height"].as_u64(),
                })
            })
        },
    )
}

/// Creates a config for fetching mainnet block data from api.blockchair.com.
pub fn endpoint_bitcoin_mainnet_api_blockchair_com() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://api.blockchair.com/bitcoin/stats",
        Some(TransformFnWrapper {
            name: "transform_bitcoin_mainnet_api_blockchair_com",
            func: transform_bitcoin_mainnet_api_blockchair_com,
        }),
        |raw| {
            apply_to_body_json(raw, |json| {
                let data = json["data"].clone();
                json!({
                    "height": data["best_block_height"].as_u64(),
                })
            })
        },
    )
}

/// Creates a config for fetching mainnet block data from api.blockcypher.com.
pub fn endpoint_bitcoin_mainnet_api_blockcypher_com() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://api.blockcypher.com/v1/btc/main",
        Some(TransformFnWrapper {
            name: "transform_bitcoin_mainnet_api_blockcypher_com",
            func: transform_bitcoin_mainnet_api_blockcypher_com,
        }),
        |raw| {
            apply_to_body_json(raw, |json| {
                json!({
                    "height": json["height"].as_u64(),
                })
            })
        },
    )
}

/// Creates a config for fetching mainnet height data from blockchain.info.
pub fn endpoint_bitcoin_mainnet_blockchain_info() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://blockchain.info/q/getblockcount",
        Some(TransformFnWrapper {
            name: "transform_bitcoin_mainnet_blockchain_info",
            func: transform_bitcoin_mainnet_blockchain_info,
        }),
        |raw| {
            apply_to_body(raw, |text| {
                text.parse::<u64>()
                    .map(|height| {
                        json!({
                            "height": height,
                        })
                        .to_string()
                    })
                    .unwrap_or_default()
            })
        },
    )
}

/// Creates a config for fetching mainnet height data from blockstream.info.
pub fn endpoint_bitcoin_mainnet_blockstream_info() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://blockstream.info/api/blocks/tip/height",
        Some(TransformFnWrapper {
            name: "transform_bitcoin_mainnet_blockstream_info",
            func: transform_bitcoin_mainnet_blockstream_info,
        }),
        |raw| {
            apply_to_body(raw, |text| {
                text.parse::<u64>()
                    .map(|height| {
                        json!({
                            "height": height,
                        })
                        .to_string()
                    })
                    .unwrap_or_default()
            })
        },
    )
}

/// Creates a config for fetching mainnet height data from mempool.space.
pub fn endpoint_bitcoin_mempool(network: Network) -> HttpRequestConfig {
    let url = match network {
        Network::BitcoinMainnet => "https://mempool.space/api/blocks/tip/height",
        Network::BitcoinTestnet => "https://mempool.space/testnet4/api/blocks/tip/height",
        _ => panic!("mempool explorer unsupported network: {:?}", network),
    };
    HttpRequestConfig::new(
        url,
        Some(TransformFnWrapper {
            name: "transform_bitcoin_mempool",
            func: transform_bitcoin_mempool,
        }),
        |raw| {
            apply_to_body(raw, |text| {
                text.parse::<u64>()
                    .map(|height| {
                        json!({
                            "height": height,
                        })
                        .to_string()
                    })
                    .unwrap_or_default()
            })
        },
    )
}

/// Creates a config for fetching mainnet block data from api.blockcypher.com.
pub fn endpoint_bitcoin_mainnet_mempool() -> HttpRequestConfig {
    endpoint_bitcoin_mempool(Network::BitcoinMainnet)
}

/// Creates a config for fetching testnet block data from api.blockcypher.com.
pub fn endpoint_bitcoin_testnet_mempool() -> HttpRequestConfig {
    endpoint_bitcoin_mempool(Network::BitcoinTestnet)
}

/// Creates a config for fetching Dogecoin mainnet block data from api.blockchair.com.
pub fn endpoint_dogecoin_mainnet_api_blockchair_com() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://api.blockchair.com/dogecoin/stats",
        Some(TransformFnWrapper {
            name: "transform_dogecoin_mainnet_api_blockchair_com",
            func: transform_dogecoin_mainnet_api_blockchair_com,
        }),
        |raw| {
            apply_to_body_json(raw, |json| {
                let data = json["data"].clone();
                json!({
                    "height": data["best_block_height"].as_u64(),
                })
            })
        },
    )
}

/// Creates a config for fetching Dogecoin mainnet block data from api.blockcypher.com.
pub fn endpoint_dogecoin_mainnet_api_blockcypher_com() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://api.blockcypher.com/v1/doge/main",
        Some(TransformFnWrapper {
            name: "transform_dogecoin_mainnet_api_blockcypher_com",
            func: transform_dogecoin_mainnet_api_blockcypher_com,
        }),
        |raw| {
            apply_to_body_json(raw, |json| {
                json!({
                    "height": json["height"].as_u64(),
                })
            })
        },
    )
}

/// Creates a config for fetching Dogecoin mainnet block data from doge.tokenview.io.
pub fn endpoint_dogecoin_mainnet_tokenview() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://doge.tokenview.io/api/chainstat/doge",
        Some(TransformFnWrapper {
            name: "transform_dogecoin_mainnet_tokenview",
            func: transform_dogecoin_mainnet_tokenview,
        }),
        |raw| {
            apply_to_body_json(raw, |json| {
                let data = json["data"].clone();
                match data["block_no"]
                    .as_u64()
                    .or_else(|| data["block_no"].as_str()?.parse().ok())
                {
                    Some(h) => json!({"height": h}),
                    None => json!({}),
                }
            })
        },
    )
}

/// Applies the given transformation function to the body of the response.
fn apply_to_body(raw: TransformArgs, f: impl FnOnce(String) -> String) -> HttpRequestResult {
    let mut response = HttpRequestResult {
        status: raw.response.status.clone(),
        ..Default::default()
    };
    if response.status == 200u8 {
        match String::from_utf8(raw.response.body) {
            Err(e) => {
                print(&format!("Failed to parse response body: err = {:?}", e));
            }
            Ok(original) => {
                let transformed = f(original);
                response.body = transformed.into_bytes();
            }
        }
    } else {
        print(&format!("Received an error: err = {:?}", raw));
    }
    response
}

/// Applies the given transformation function to the JSON inside the body of the response.
fn apply_to_body_json(
    raw: TransformArgs,
    f: impl FnOnce(serde_json::Value) -> serde_json::Value,
) -> HttpRequestResult {
    apply_to_body(raw, |text| match serde_json::from_str(&text) {
        Err(e) => {
            print(&format!(
                "Failed to parse JSON in response body, error: {e:?}, text: {text:?}"
            ));
            String::default()
        }
        Ok(original) => {
            let transformed = f(original);
            transformed.to_string()
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils;
    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    const ZERO_CYCLES: u128 = 0;

    fn parse_json(body: Vec<u8>) -> serde_json::Value {
        let json_str = String::from_utf8(body).expect("Raw response is not UTF-8 encoded.");
        serde_json::from_str(&json_str).expect("Failed to parse JSON from string")
    }

    async fn run_http_request_test(
        config: HttpRequestConfig,
        expected_url: &str,
        mock_response_body: &str,
        expected: serde_json::Value,
    ) {
        let request = config.request();
        let mock_response = ic_http::create_response()
            .status(200)
            .body(mock_response_body)
            .build();
        ic_http::mock::mock(request.clone(), mock_response);

        let response = ic_http::http_request(request.clone(), ZERO_CYCLES)
            .await
            .expect("HTTP request failed");

        assert_eq!(config.url(), expected_url.to_string());
        assert_json_eq!(parse_json(response.body), expected);
        assert_eq!(ic_http::mock::times_called(request), 1);
    }

    #[tokio::test]
    async fn test_api_bitaps_com_block_mainnet() {
        run_http_request_test(
            endpoint_bitcoin_mainnet_api_bitaps_com(),
            "https://api.bitaps.com/btc/v1/blockchain/block/last",
            test_utils::BITCOIN_MAINNET_API_BITAPS_COM_RESPONSE,
            json!({
                "height": 700001,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_api_blockchair_com_block() {
        run_http_request_test(
            endpoint_bitcoin_mainnet_api_blockchair_com(),
            "https://api.blockchair.com/bitcoin/stats",
            test_utils::BITCOIN_MAINNET_API_BLOCKCHAIR_COM_RESPONSE,
            json!({
                "height": 700002,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_api_blockcypher_com_block() {
        run_http_request_test(
            endpoint_bitcoin_mainnet_api_blockcypher_com(),
            "https://api.blockcypher.com/v1/btc/main",
            test_utils::BITCOIN_MAINNET_API_BLOCKCYPHER_COM_RESPONSE,
            json!({
                "height": 700003,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_blockchain_info_height() {
        run_http_request_test(
            endpoint_bitcoin_mainnet_blockchain_info(),
            "https://blockchain.info/q/getblockcount",
            test_utils::BITCOIN_MAINNET_BLOCKCHAIN_INFO_RESPONSE,
            json!({
                "height": 700004,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_blockstream_info_height() {
        run_http_request_test(
            endpoint_bitcoin_mainnet_blockstream_info(),
            "https://blockstream.info/api/blocks/tip/height",
            test_utils::BITCOIN_MAINNET_BLOCKSTREAM_INFO_RESPONSE,
            json!({
                "height": 700005,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_dogecoin_api_blockchair_com_block() {
        run_http_request_test(
            endpoint_dogecoin_mainnet_api_blockchair_com(),
            "https://api.blockchair.com/dogecoin/stats",
            test_utils::DOGECOIN_MAINNET_API_BLOCKCHAIR_COM_RESPONSE,
            json!({
                "height": 5926987,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_dogecoin_api_blockcypher_com_block() {
        run_http_request_test(
            endpoint_dogecoin_mainnet_api_blockcypher_com(),
            "https://api.blockcypher.com/v1/doge/main",
            test_utils::DOGECOIN_MAINNET_API_BLOCKCYPHER_COM_RESPONSE,
            json!({
                "height": 5926989,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_dogecoin_tokenview_height_mainnet() {
        run_http_request_test(
            endpoint_dogecoin_mainnet_tokenview(),
            "https://doge.tokenview.io/api/chainstat/doge",
            test_utils::DOGECOIN_MAINNET_TOKENVIEW_RESPONSE,
            json!({
                "height": 5931072,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_mempool_height_mainnet() {
        run_http_request_test(
            endpoint_bitcoin_mainnet_mempool(),
            "https://mempool.space/api/blocks/tip/height",
            test_utils::BITCOIN_MAINNET_MEMPOOL_RESPONSE,
            json!({
                "height": 700008,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_mempool_height_testnet() {
        run_http_request_test(
            endpoint_bitcoin_testnet_mempool(),
            "https://mempool.space/testnet4/api/blocks/tip/height",
            test_utils::BITCOIN_TESTNET_MEMPOOL_RESPONSE,
            json!({
                "height": 55002,
            }),
        )
        .await;
    }

    #[test]
    fn test_transform_function_names() {
        test_utils::mock_bitcoin_mainnet_outcalls();
        test_utils::mock_bitcoin_testnet_outcalls();
        test_utils::mock_dogecoin_mainnet_outcalls();

        let names = ic_http::mock::registered_transform_function_names();
        let names = names.iter().map(|s| s.as_str()).collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "transform_bitcoin_mainnet_api_bitaps_com",
                "transform_bitcoin_mainnet_api_blockchair_com",
                "transform_bitcoin_mainnet_api_blockcypher_com",
                "transform_bitcoin_mainnet_blockchain_info",
                "transform_bitcoin_mainnet_blockstream_info",
                "transform_bitcoin_mempool",
                "transform_dogecoin_mainnet_api_blockchair_com",
                "transform_dogecoin_mainnet_api_blockcypher_com",
                "transform_dogecoin_mainnet_tokenview",
            ]
        );
    }

    #[tokio::test]
    async fn test_http_response_404() {
        let expected_status = candid::Nat::from(404u16);
        let test_cases = [
            endpoint_bitcoin_mainnet_api_blockchair_com(),
            endpoint_bitcoin_mainnet_api_blockcypher_com(),
            endpoint_bitcoin_mainnet_blockchain_info(),
            endpoint_bitcoin_mainnet_blockstream_info(),
            endpoint_bitcoin_testnet_mempool(),
            endpoint_dogecoin_mainnet_api_blockchair_com(),
            endpoint_dogecoin_mainnet_api_blockcypher_com(),
            endpoint_dogecoin_mainnet_tokenview(),
        ];
        for config in test_cases {
            // Arrange
            let request = config.request();
            let mock_response = ic_http::create_response().status(404).build();
            ic_http::mock::mock(request.clone(), mock_response);

            // Act
            let response = ic_http::http_request(request, ZERO_CYCLES).await.unwrap();

            // Assert
            assert_eq!(response.status, expected_status, "url: {:?}", config.url());
            assert_eq!(response.body, Vec::<u8>::new(), "url: {:?}", config.url());
        }
    }
}
