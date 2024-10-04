use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::config::BitcoinNetwork;
use candid::CandidType;
use serde::{Deserialize, Serialize};

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
pub async fn fetch_all_data(bitcoin_network: BitcoinNetwork) -> Vec<BlockInfo> {
    let api_providers = BitcoinBlockApi::network_providers(bitcoin_network);

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
    async fn test_fetch_all_data_mainnet() {
        crate::storage::set_config(crate::config::Config::mainnet());
        crate::test_utils::mock_mainnet_outcalls();

        let result = fetch_all_data(BitcoinNetwork::Mainnet).await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBitapsComMainnet,
                    height: Some(700001),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockchairComMainnet,
                    height: Some(700002),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockcypherComMainnet,
                    height: Some(700003),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BitcoinExplorerOrgMainnet,
                    height: Some(861687),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockchainInfoMainnet,
                    height: Some(700004),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockstreamInfoMainnet,
                    height: Some(700005),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ChainApiBtcComMainnet,
                    height: Some(700006),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BitcoinCanister,
                    height: Some(700007),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_testnet() {
        crate::storage::set_config(crate::config::Config::testnet());
        crate::test_utils::mock_testnet_outcalls();

        let result = fetch_all_data(BitcoinNetwork::Testnet).await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBitapsComTestnet,
                    height: Some(2000001),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockchairComTestnet,
                    height: Some(2000002),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockcypherComTestnet,
                    height: Some(2000003),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockstreamInfoTestnet,
                    height: Some(2000004),
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BitcoinCanister,
                    height: Some(2000007),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_mainnet() {
        crate::storage::set_config(crate::config::Config::mainnet());
        crate::test_utils::mock_all_outcalls_404();

        let result = fetch_all_data(BitcoinNetwork::Mainnet).await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBitapsComMainnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockchairComMainnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockcypherComMainnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BitcoinExplorerOrgMainnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockchainInfoMainnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockstreamInfoMainnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ChainApiBtcComMainnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BitcoinCanister,
                    height: None,
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_testnet() {
        crate::storage::set_config(crate::config::Config::testnet());
        crate::test_utils::mock_all_outcalls_404();

        let result = fetch_all_data(BitcoinNetwork::Testnet).await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBitapsComTestnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockchairComTestnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::ApiBlockcypherComTestnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BlockstreamInfoTestnet,
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinBlockApi::BitcoinCanister,
                    height: None,
                },
            ]
        );
    }
}
