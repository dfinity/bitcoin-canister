use crate::http::HttpRequestConfig;
use crate::print;
use crate::{
    transform_api_bitaps_com_block, transform_api_blockchair_com_block,
    transform_api_blockcypher_com_block, transform_bitcoin_canister,
    transform_blockchain_info_hash, transform_blockchain_info_height,
    transform_blockstream_info_hash, transform_blockstream_info_height,
    transform_chain_api_btc_com_block,
};
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use regex::Regex;
use serde_json::json;

/// Creates a new HttpRequestConfig for fetching block data from api.bitaps.com.
pub fn endpoint_api_bitaps_com_block() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://api.bitaps.com/btc/v1/blockchain/block/last",
        Some(transform_api_bitaps_com_block),
        |raw| {
            apply_to_body_json(raw, |json| {
                let data = json["data"].clone();
                json!({
                    "height": data["height"].as_u64(),
                    "hash": data["hash"].as_str(),
                })
            })
        },
    )
}

/// Creates a new HttpRequestConfig for fetching block data from api.blockchair.com.
pub fn endpoint_api_blockchair_com_block() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://api.blockchair.com/bitcoin/stats",
        Some(transform_api_blockchair_com_block),
        |raw| {
            apply_to_body_json(raw, |json| {
                let data = json["data"].clone();
                json!({
                    "height": data["best_block_height"].as_u64(),
                    "hash": data["best_block_hash"].as_str(),
                })
            })
        },
    )
}

/// Creates a new HttpRequestConfig for fetching block data from api.blockcypher.com.
pub fn endpoint_api_blockcypher_com_block() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://api.blockcypher.com/v1/btc/main",
        Some(transform_api_blockcypher_com_block),
        |raw| {
            apply_to_body_json(raw, |json| {
                json!({
                    "height": json["height"].as_u64(),
                    "hash": json["hash"].as_str(),
                    "previous_hash": json["previous_hash"].as_str(),
                })
            })
        },
    )
}

/// Applies regex rule to parse bitcoin_canister block height.
fn regex_height(text: String) -> Result<String, String> {
    const RE_PATTERN: &str = r"\s*main_chain_height (\d+) \d+";
    match Regex::new(RE_PATTERN) {
        Err(e) => Err(format!("Regex: failed to compile: {}", e)),
        Ok(re) => match re.captures(&text) {
            None => Err("Regex: no match found.".to_string()),
            Some(cap) => match cap.len() {
                2 => Ok(String::from(&cap[1])),
                x => Err(format!("Regex: expected 1 group exactly, provided {}.", x)),
            },
        },
    }
}

/// Parses text for bitcoin_canister block height.
fn parse_bitcoin_canister_height(text: String) -> Result<u64, String> {
    let height = regex_height(text)?;
    match height.parse::<u64>() {
        Ok(height) => Ok(height),
        Err(_) => Err(format!("Failed to parse height: {}", height)),
    }
}

/// Creates a new HttpRequestConfig for fetching block data from bitcoin_canister.
pub fn endpoint_bitcoin_canister() -> HttpRequestConfig {
    HttpRequestConfig::new(
        crate::config::BITCOIN_CANISTER_ENDPOINT,
        Some(transform_bitcoin_canister),
        |raw| {
            apply_to_body(raw, |text| {
                parse_bitcoin_canister_height(text)
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

/// Creates a new HttpRequestConfig for fetching hash data from blockchain.info.
pub fn endpoint_blockchain_info_hash() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://blockchain.info/q/latesthash",
        Some(transform_blockchain_info_hash),
        |raw| {
            apply_to_body(raw, |text| {
                json!({
                    "hash": text,
                })
                .to_string()
            })
        },
    )
}

/// Creates a new HttpRequestConfig for fetching height data from blockchain.info.
pub fn endpoint_blockchain_info_height() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://blockchain.info/q/getblockcount",
        Some(transform_blockchain_info_height),
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

/// Creates a new HttpRequestConfig for fetching hash data from blockstream.info.
pub fn endpoint_blockstream_info_hash() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://blockstream.info/api/blocks/tip/hash",
        Some(transform_blockstream_info_hash),
        |raw| {
            apply_to_body(raw, |text| {
                json!({
                    "hash": text,
                })
                .to_string()
            })
        },
    )
}

/// Creates a new HttpRequestConfig for fetching height data from blockstream.info.
pub fn endpoint_blockstream_info_height() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://blockstream.info/api/blocks/tip/height",
        Some(transform_blockstream_info_height),
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

/// Creates a new HttpRequestConfig for fetching block data from chain.api.btc.com.
pub fn endpoint_chain_api_btc_com_block() -> HttpRequestConfig {
    HttpRequestConfig::new(
        "https://chain.api.btc.com/v3/block/latest",
        Some(transform_chain_api_btc_com_block),
        |raw| {
            apply_to_body_json(raw, |json| {
                let data = json["data"].clone();
                json!({
                    "height": data["height"].as_u64(),
                    "hash": data["hash"].as_str(),
                    "previous_hash": data["prev_block_hash"].as_str(),
                })
            })
        },
    )
}

/// Applies the given transformation function to the body of the response.
fn apply_to_body(raw: TransformArgs, f: impl FnOnce(String) -> String) -> HttpResponse {
    let mut response = HttpResponse {
        status: raw.response.status.clone(),
        ..Default::default()
    };
    if response.status == 200 {
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
) -> HttpResponse {
    apply_to_body(raw, |text| match serde_json::from_str(&text) {
        Err(e) => {
            print(&format!("Failed to parse response body: err = {:?}", e));
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

        let (response,) = ic_http::http_request(request.clone())
            .await
            .expect("HTTP request failed");

        assert_eq!(config.url(), expected_url.to_string());
        assert_json_eq!(parse_json(response.body), expected);
        assert_eq!(ic_http::mock::times_called(request), 1);
    }

    #[tokio::test]
    async fn test_api_bitaps_com_block() {
        run_http_request_test(
            endpoint_api_bitaps_com_block(),
            "https://api.bitaps.com/btc/v1/blockchain/block/last",
            test_utils::API_BITAPS_COM_RESPONSE,
            json!({
                "height": 700001,
                "hash": "0000000000000000000aaa111111111111111111111111111111111111111111",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_api_blockchair_com_block() {
        run_http_request_test(
            endpoint_api_blockchair_com_block(),
            "https://api.blockchair.com/bitcoin/stats",
            test_utils::API_BLOCKCHAIR_COM_RESPONSE,
            json!({
                "height": 700002,
                "hash": "0000000000000000000aaa222222222222222222222222222222222222222222",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_api_blockcypher_com_block() {
        run_http_request_test(
            endpoint_api_blockcypher_com_block(),
            "https://api.blockcypher.com/v1/btc/main",
            test_utils::API_BLOCKCYPHER_COM_RESPONSE,
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
        run_http_request_test(
            endpoint_bitcoin_canister(),
            "https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics",
            test_utils::BITCOIN_CANISTER_RESPONSE,
            json!({
                "height": 700007,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_blockchain_info_hash() {
        run_http_request_test(
            endpoint_blockchain_info_hash(),
            "https://blockchain.info/q/latesthash",
            test_utils::BLOCKCHAIN_INFO_HASH_RESPONSE,
            json!({
                "hash": "0000000000000000000aaa444444444444444444444444444444444444444444",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_blockchain_info_height() {
        run_http_request_test(
            endpoint_blockchain_info_height(),
            "https://blockchain.info/q/getblockcount",
            test_utils::BLOCKCHAIN_INFO_HEIGHT_RESPONSE,
            json!({
                "height": 700004,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_blockstream_info_hash() {
        run_http_request_test(
            endpoint_blockstream_info_hash(),
            "https://blockstream.info/api/blocks/tip/hash",
            test_utils::BLOCKSTREAM_INFO_HASH_RESPONSE,
            json!({
                "hash": "0000000000000000000aaa555555555555555555555555555555555555555555",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_blockstream_info_height() {
        run_http_request_test(
            endpoint_blockstream_info_height(),
            "https://blockstream.info/api/blocks/tip/height",
            test_utils::BLOCKSTREAM_INFO_HEIGHT_RESPONSE,
            json!({
                "height": 700005,
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_chain_api_btc_com_block() {
        run_http_request_test(
            endpoint_chain_api_btc_com_block(),
            "https://chain.api.btc.com/v3/block/latest",
            test_utils::CHAIN_API_BTC_COM_RESPONSE,
            json!({
                "height": 700006,
                "hash": "0000000000000000000aaa666666666666666666666666666666666666666666",
                "previous_hash": "0000000000000000000aaa555555555555555555555555555555555555555555",
            }),
        )
        .await;
    }

    #[test]
    fn test_transform_function_names() {
        test_utils::mock_all_outcalls();

        let names = ic_http::mock::registered_transform_function_names();
        let names = names.iter().map(|s| s.as_str()).collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "transform_api_bitaps_com_block",
                "transform_api_blockchair_com_block",
                "transform_api_blockcypher_com_block",
                "transform_bitcoin_canister",
                "transform_blockchain_info_hash",
                "transform_blockchain_info_height",
                "transform_blockstream_info_hash",
                "transform_blockstream_info_height",
                "transform_chain_api_btc_com_block"
            ]
        );
    }

    #[test]
    fn test_height_regex() {
        let test_cases = [
            (r#"main_chain_height 700007 1680014894644"#, Ok(700007)),
            (
                r#"    main_chain_height 700007 1680014894644   "#,
                Ok(700007),
            ),
            (
                r#"
                    # HELP main_chain_height Height of the main chain.
                    # TYPE main_chain_height gauge
                    main_chain_height 700007 1680014894644
                    # HELP stable_height The height of the latest stable block.
                    # TYPE stable_height gauge
                    stable_height 782801 1680014894644
                "#,
                Ok(700007),
            ),
            (r#"700007"#, Err("Regex: no match found.".to_string())),
            (
                r#"main_chain_height 123456789012345678901234567890 123"#,
                Err("Failed to parse height: 123456789012345678901234567890".to_string()),
            ),
        ];

        for (text, expected) in test_cases {
            let result = parse_bitcoin_canister_height(text.to_string());
            assert_eq!(result, expected);
        }
    }

    #[tokio::test]
    async fn test_http_response_404() {
        let expected_status = candid::Nat::from(404);
        let test_cases = [
            endpoint_api_bitaps_com_block(),
            endpoint_api_blockchair_com_block(),
            endpoint_api_blockcypher_com_block(),
            endpoint_bitcoin_canister(),
            endpoint_blockchain_info_hash(),
            endpoint_blockchain_info_height(),
            endpoint_blockstream_info_hash(),
            endpoint_blockstream_info_height(),
            endpoint_chain_api_btc_com_block(),
        ];
        for config in test_cases {
            // Arrange
            let request = config.request();
            let mock_response = ic_http::create_response().status(404).build();
            ic_http::mock::mock(request.clone(), mock_response);

            // Act
            let (response,) = ic_http::http_request(request).await.unwrap();

            // Assert
            assert_eq!(response.status, expected_status, "url: {:?}", config.url());
            assert_eq!(response.body, Vec::<u8>::new(), "url: {:?}", config.url());
        }
    }
}
