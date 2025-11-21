use crate::block_apis::{BlockApi, CandidBlockApi};
use crate::config::Network;
use candid::CandidType;
use serde::{Deserialize, Serialize};

/// The data fetched from the external block APIs.
#[derive(Clone, Debug, Eq, PartialEq, CandidType, Serialize, Deserialize)]
pub struct BlockInfo {
    /// The provider of the block data.
    pub provider: CandidBlockApi,

    /// The height of the block.
    pub height: Option<u64>,
}

impl BlockInfo {
    #[cfg(test)]
    pub fn new(provider: impl Into<CandidBlockApi>, height: u64) -> Self {
        Self {
            provider: provider.into(),
            height: Some(height),
        }
    }
}

/// Fetches the data from the external APIs.
pub async fn fetch_all_data(network: Network) -> Vec<BlockInfo> {
    let api_providers = BlockApi::network_providers(network);

    let futures = api_providers
        .iter()
        .map(|api| api.fetch_data())
        .collect::<Vec<_>>();
    let results = futures::future::join_all(futures).await;

    let result: Vec<_> = api_providers
        .iter()
        .zip(results.iter())
        .map(|(api, value)| BlockInfo {
            provider: CandidBlockApi::from(api.clone()),
            height: value["height"].as_u64(),
        })
        .collect();

    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block_apis::{
        BitcoinMainnetExplorerBlockApi, BitcoinProviderBlockApi, BitcoinTestnetExplorerBlockApi,
        DogecoinMainnetExplorerBlockApi, DogecoinProviderBlockApi,
    };
    use crate::config::Canister;
    use crate::fetch::BlockInfo;

    #[tokio::test]
    async fn test_fetch_all_data_bitcoin_mainnet() {
        crate::storage::set_config(crate::config::Config::for_target(Canister::BitcoinMainnet));
        crate::test_utils::mock_bitcoin_mainnet_outcalls();

        verify_bitcoin_mainnet_fetch_results().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_bitcoin_mainnet_staging() {
        crate::storage::set_config(crate::config::Config::for_target(
            Canister::BitcoinMainnetStaging,
        ));
        crate::test_utils::mock_bitcoin_mainnet_outcalls();

        verify_bitcoin_mainnet_fetch_results().await;
    }

    async fn verify_bitcoin_mainnet_fetch_results() {
        let result = fetch_all_data(Network::BitcoinMainnet).await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBitapsCom.into(),
                    height: Some(700001),
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    height: Some(700002),
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    height: Some(700003),
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                    height: Some(700004),
                },
                // BlockInfo {
                //     provider: BitcoinMainnetExplorerBlockApi::BlockexplorerOne.into(),
                //     height: Some(923450),
                // }, // TODO(DEFI-2493): add BlockexplorerOne
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                    height: Some(700005),
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::Mempool.into(),
                    height: Some(700008),
                },
                BlockInfo {
                    provider: BitcoinProviderBlockApi::BitcoinCanister.into(),
                    height: Some(700007),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_testnet() {
        crate::storage::set_config(crate::config::Config::for_target(Canister::BitcoinTestnet));
        crate::test_utils::mock_bitcoin_testnet_outcalls();

        let result = fetch_all_data(Network::BitcoinTestnet).await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: BitcoinTestnetExplorerBlockApi::Mempool.into(),
                    height: Some(55002),
                },
                BlockInfo {
                    provider: BitcoinProviderBlockApi::BitcoinCanister.into(),
                    height: Some(55001),
                },
            ]
        );
    }

    async fn verify_dogecoin_mainnet_fetch_results() {
        let result = fetch_all_data(Network::DogecoinMainnet).await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: DogecoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    height: Some(5926987),
                },
                BlockInfo {
                    provider: DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    height: Some(5926989),
                },
                BlockInfo {
                    provider: DogecoinMainnetExplorerBlockApi::TokenView.into(),
                    height: Some(5931072),
                },
                BlockInfo {
                    provider: DogecoinProviderBlockApi::DogecoinCanister.into(),
                    height: Some(5931098),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_dogecoin_mainnet() {
        crate::storage::set_config(crate::config::Config::for_target(Canister::DogecoinMainnet));
        crate::test_utils::mock_dogecoin_mainnet_outcalls();

        verify_dogecoin_mainnet_fetch_results().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_dogecoin_mainnet_staging() {
        crate::storage::set_config(crate::config::Config::for_target(
            Canister::DogecoinMainnetStaging,
        ));
        crate::test_utils::mock_dogecoin_mainnet_outcalls();

        verify_dogecoin_mainnet_fetch_results().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_mainnet() {
        crate::storage::set_config(crate::config::Config::for_target(Canister::BitcoinMainnet));
        crate::test_utils::mock_all_outcalls_404();

        verify_bitcoin_mainnet_fetch_failed_404().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_mainnet_staging() {
        crate::storage::set_config(crate::config::Config::for_target(
            Canister::BitcoinMainnetStaging,
        ));
        crate::test_utils::mock_all_outcalls_404();

        verify_bitcoin_mainnet_fetch_failed_404().await;
    }

    async fn verify_bitcoin_mainnet_fetch_failed_404() {
        let result = fetch_all_data(Network::BitcoinMainnet).await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBitapsCom.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                    height: None,
                },
                // BlockInfo {
                //     provider: BitcoinMainnetExplorerBlockApi::BlockexplorerOne.into(),
                //     height: None,
                // }, // TODO(DEFI-2493): add BlockexplorerOne
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::Mempool.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinProviderBlockApi::BitcoinCanister.into(),
                    height: None,
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_testnet() {
        crate::storage::set_config(crate::config::Config::for_target(Canister::BitcoinTestnet));
        crate::test_utils::mock_all_outcalls_404();

        let result = fetch_all_data(Network::BitcoinTestnet).await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: BitcoinTestnetExplorerBlockApi::Mempool.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinProviderBlockApi::BitcoinCanister.into(),
                    height: None,
                },
            ]
        );
    }

    async fn verify_dogecoin_mainnet_fetch_failed_404() {
        let result = fetch_all_data(Network::DogecoinMainnet).await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: DogecoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    height: None,
                },
                BlockInfo {
                    provider: DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    height: None,
                },
                BlockInfo {
                    provider: DogecoinMainnetExplorerBlockApi::TokenView.into(),
                    height: None,
                },
                BlockInfo {
                    provider: DogecoinProviderBlockApi::DogecoinCanister.into(),
                    height: None,
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_dogecoin_mainnet() {
        crate::storage::set_config(crate::config::Config::for_target(Canister::DogecoinMainnet));
        crate::test_utils::mock_all_outcalls_404();

        verify_dogecoin_mainnet_fetch_failed_404().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_dogecoin_mainnet_staging() {
        crate::storage::set_config(crate::config::Config::for_target(
            Canister::DogecoinMainnetStaging,
        ));
        crate::test_utils::mock_all_outcalls_404();

        verify_dogecoin_mainnet_fetch_failed_404().await;
    }
}
