use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::bitcoin_block_apis::BitcoinBlockApi;

/// The data fetched from the external bitcoin block APIs.
#[derive(Clone, Debug, Eq, PartialEq, CandidType, Serialize, Deserialize)]
pub struct BlockInfo {
    /// The provider of the bitcoin block data.
    pub provider: BitcoinBlockApi,

    /// The height of the block.
    pub height: Option<u64>,
}

impl BlockInfo {
    #[cfg(test)]
    pub fn new(provider: BitcoinBlockApi, height: u64) -> Self {
        Self {
            provider,
            height: Some(height),
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
                    provider: BitcoinBlockApi::ApiBlockchairCom,
                    height: Some(700002),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockcypherCom,
                    height: Some(700003),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BitcoinCanister,
                    height: Some(700007),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockchainInfo,
                    height: Some(700004),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockstreamInfo,
                    height: Some(700005),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ChainApiBtcCom,
                    height: Some(700006),
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
                    provider: BitcoinBlockApi::ApiBlockchairCom,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockcypherCom,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BitcoinCanister,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockchainInfo,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockstreamInfo,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ChainApiBtcCom,
                    height: None,
                }
            ]
        );
    }
}
