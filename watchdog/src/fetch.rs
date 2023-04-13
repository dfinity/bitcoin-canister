use crate::bitcoin_block_apis::BitcoinBlockApi;

/// The data fetched from the external bitcoin block APIs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockInfo {
    pub provider: BitcoinBlockApi,
    pub height: Option<u64>,
    hash: Option<String>,
    previous_hash: Option<String>,
}

impl BlockInfo {
    #[cfg(test)]
    pub fn new(provider: BitcoinBlockApi, height: u64) -> Self {
        Self {
            provider,
            height: Some(height),
            hash: None,
            previous_hash: None,
        }
    }
}

/// Fetches the data from the external APIs.
pub async fn fetch_all_data() -> Vec<BlockInfo> {
    let api_providers = BitcoinBlockApi::all_providers();

    let futures = api_providers
        .iter()
        .map(|api| api.fetch_data())
        .collect::<Vec<_>>();
    let results = futures::future::join_all(futures).await;

    let result: Vec<_> = api_providers
        .iter()
        .zip(results.iter())
        .map(|(api, value)| BlockInfo {
            provider: api.clone(),
            height: value["height"].as_u64(),
            hash: value["hash"].as_str().map(|s| s.to_string()),
            previous_hash: value["previous_hash"].as_str().map(|s| s.to_string()),
        })
        .collect();

    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::fetch::BlockInfo;

    #[tokio::test]
    async fn test_fetch_all_data() {
        crate::test_utils::mock_all_outcalls();

        let result = fetch_all_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBitapsCom,
                    height: Some(700001),
                    hash: Some(
                        "0000000000000000000aaa111111111111111111111111111111111111111111"
                            .to_string()
                    ),
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockchairCom,
                    height: Some(700002),
                    hash: Some(
                        "0000000000000000000aaa222222222222222222222222222222222222222222"
                            .to_string()
                    ),
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockcypherCom,
                    height: Some(700003),
                    hash: Some(
                        "0000000000000000000aaa333333333333333333333333333333333333333333"
                            .to_string()
                    ),
                    previous_hash: Some(
                        "0000000000000000000aaa222222222222222222222222222222222222222222"
                            .to_string()
                    )
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BitcoinCanister,
                    height: Some(700007),
                    hash: None,
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockchainInfo,
                    height: Some(700004),
                    hash: Some(
                        "0000000000000000000aaa444444444444444444444444444444444444444444"
                            .to_string()
                    ),
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockstreamInfo,
                    height: Some(700005),
                    hash: Some(
                        "0000000000000000000aaa555555555555555555555555555555555555555555"
                            .to_string()
                    ),
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ChainApiBtcCom,
                    height: Some(700006),
                    hash: Some(
                        "0000000000000000000aaa666666666666666666666666666666666666666666"
                            .to_string()
                    ),
                    previous_hash: Some(
                        "0000000000000000000aaa555555555555555555555555555555555555555555"
                            .to_string()
                    )
                }
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404() {
        crate::test_utils::mock_all_outcalls_404();

        let result = fetch_all_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBitapsCom,
                    height: None,
                    hash: None,
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockchairCom,
                    height: None,
                    hash: None,
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockcypherCom,
                    height: None,
                    hash: None,
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BitcoinCanister,
                    height: None,
                    hash: None,
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockchainInfo,
                    height: None,
                    hash: None,
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockstreamInfo,
                    height: None,
                    hash: None,
                    previous_hash: None
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ChainApiBtcCom,
                    height: None,
                    hash: None,
                    previous_hash: None
                }
            ]
        );
    }
}
