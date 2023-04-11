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

#[derive(Debug)]
pub enum Endpoint {
    ApiBitapsComBlock,
    ApiBlockchairComBlock,
    ApiBlockcypherComBlock,
    BitcoinCanister,
    BlockchainInfoHash,
    BlockchainInfoHeight,
    BlockstreamInfoHash,
    BlockstreamInfoHeight,
    ChainApiBtcComBlock,
}

impl Endpoint {
    pub fn get(&self) -> HttpRequestConfig {
        match self {
            Endpoint::ApiBitapsComBlock => HttpRequestConfig::new(
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
            ),
            Endpoint::ApiBlockchairComBlock => HttpRequestConfig::new(
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
            ),
            Endpoint::ApiBlockcypherComBlock => HttpRequestConfig::new(
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
            ),
            Endpoint::BitcoinCanister => HttpRequestConfig::new(
                "https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics",
                Some(transform_bitcoin_canister),
                |raw| {
                    apply_to_body(raw, |text| {
                        const RE_PATTERN: &str = r"\n\s*main_chain_height (\d+) \d+\n";
                        let re = Regex::new(RE_PATTERN).unwrap();
                        let height: u64 = match apply_regex(&re, &text) {
                            Err(_) => panic!("oops"),
                            Ok(height) => height.parse::<u64>().unwrap(),
                        };
                        json!({
                            "height": height,
                        })
                        .to_string()
                    })
                },
            ),
            Endpoint::BlockchainInfoHash => HttpRequestConfig::new(
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
            ),
            Endpoint::BlockchainInfoHeight => HttpRequestConfig::new(
                "https://blockchain.info/q/getblockcount",
                Some(transform_blockchain_info_height),
                |raw| {
                    apply_to_body(raw, |text| {
                        json!({
                            "height": text.parse::<u64>().unwrap(),
                        })
                        .to_string()
                    })
                },
            ),
            Endpoint::BlockstreamInfoHash => HttpRequestConfig::new(
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
            ),
            Endpoint::BlockstreamInfoHeight => HttpRequestConfig::new(
                "https://blockstream.info/api/blocks/tip/height",
                Some(transform_blockstream_info_height),
                |raw| {
                    apply_to_body(raw, |text| {
                        json!({
                            "height": text.parse::<u64>().unwrap(),
                        })
                        .to_string()
                    })
                },
            ),
            Endpoint::ChainApiBtcComBlock => HttpRequestConfig::new(
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
            ),
        }
    }
}

fn apply_to_body(raw: TransformArgs, f: impl FnOnce(String) -> String) -> HttpResponse {
    let mut response = HttpResponse {
        status: raw.response.status.clone(),
        ..Default::default()
    };
    if response.status == 200 {
        let original =
            String::from_utf8(raw.response.body).expect("Raw response is not UTF-8 encoded.");
        let transformed = f(original);
        response.body = transformed.into_bytes();
    } else {
        print(&format!("Received an error: err = {:?}", raw));
    }
    response
}

fn apply_to_body_json(
    raw: TransformArgs,
    f: impl FnOnce(serde_json::Value) -> serde_json::Value,
) -> HttpResponse {
    apply_to_body(raw, |text| {
        let before = serde_json::from_str(&text).expect("Failed to parse JSON from string");
        let after = f(before);
        after.to_string()
    })
}

/// Apply regex rule to a given text.
fn apply_regex(re: &Regex, text: &str) -> Result<String, String> {
    match re.captures(text) {
        None => Err("Regex: no match found.".to_string()),
        Some(cap) => match cap.len() {
            2 => Ok(String::from(&cap[1])),
            x => Err(format!("Regex: expected 1 group exactly, provided {}.", x)),
        },
    }
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
        endpoint: Endpoint,
        url: &str,
        response_body: &str,
        expected: serde_json::Value,
    ) {
        let request = endpoint.get().create_request();
        let mock_response = ic_http::create_response()
            .status(200)
            .body(response_body)
            .build();
        ic_http::mock::mock(request.clone(), mock_response);

        let (response,) = ic_http::http_request(request.clone()).await.unwrap();

        assert_eq!(endpoint.get().url(), url);
        assert_json_eq!(parse_json(response.body), expected);
        assert_eq!(ic_http::mock::times_called(request), 1);
    }

    #[tokio::test]
    async fn test_api_bitaps_com_block() {
        run_http_request_test(
            Endpoint::ApiBitapsComBlock,
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
            Endpoint::ApiBlockchairComBlock,
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
            Endpoint::ApiBlockcypherComBlock,
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
            Endpoint::BitcoinCanister,
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
            Endpoint::BlockchainInfoHash,
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
            Endpoint::BlockchainInfoHeight,
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
            Endpoint::BlockstreamInfoHash,
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
            Endpoint::BlockstreamInfoHeight,
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
            Endpoint::ChainApiBtcComBlock,
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
}
