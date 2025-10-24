use candid::CandidType;
use serde::{Deserialize, Serialize};
use crate::bitcoin_block_apis::BlockApi;
use crate::config::Network;

/// The data fetched from the external block APIs.
#[derive(Clone, Debug, Eq, PartialEq, CandidType, Serialize, Deserialize)]
pub struct BlockInfo {
    /// The provider of the block data.
    pub provider: BlockApi,

    /// The height of the block.
    pub height: Option<u64>,
}

impl BlockInfo {
    #[cfg(test)]
    pub fn new(provider: BlockApi, height: u64) -> Self {
        Self {
            provider,
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
            provider: api.clone(),
            height: value["height"].as_u64(),
        })
        .collect();

    result
}

#[cfg(test)]
mod test {
    use crate::bitcoin_block_apis::{BitcoinMainnetExplorerBlockApi, BitcoinProviderBlockApi, BitcoinTestnetExplorerBlockApi, DogecoinMainnetExplorerBlockApi, DogecoinProviderBlockApi};
    use super::*;
    use crate::fetch::BlockInfo;

    #[tokio::test]
    async fn test_fetch_all_data_bitcoin_mainnet() {
        crate::storage::set_config(crate::config::Config::bitcoin_mainnet());
        crate::test_utils::mock_bitcoin_mainnet_outcalls();

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
                    provider: BitcoinMainnetExplorerBlockApi::BitcoinExplorerOrg.into(),
                    height: Some(861687),
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                    height: Some(700004),
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                    height: Some(700005),
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::ChainApiBtcCom.into(),
                    height: Some(700006),
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::Mempool.into(),
                    height: Some(700008),
                },
                BlockInfo {
                    provider: BlockApi::BitcoinProvider(BitcoinProviderBlockApi::BitcoinCanister),
                    height: Some(700007),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_testnet() {
        crate::storage::set_config(crate::config::Config::bitcoin_testnet());
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
                    provider: BlockApi::BitcoinProvider(BitcoinProviderBlockApi::BitcoinCanister),
                    height: Some(55001),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_dogecoin_mainnet() {
        let staging_canister = false; // This parameter is not important. TODO(mducroux): revisit.
        crate::storage::set_config(crate::config::Config::dogecoin_mainnet(staging_canister));
        crate::test_utils::mock_dogecoin_mainnet_outcalls();

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
                    provider: BlockApi::DogecoinProvider(DogecoinProviderBlockApi::DogecoinCanister),
                    height: Some(???), // TODO(mducroux): complete
                },
                BlockInfo {
                    provider: DogecoinMainnetExplorerBlockApi::TokenView.into(),
                    height: Some(5931072),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_mainnet() {
        crate::storage::set_config(crate::config::Config::bitcoin_mainnet());
        crate::test_utils::mock_all_outcalls_404();

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
                    provider: BitcoinMainnetExplorerBlockApi::BitcoinExplorerOrg.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::ChainApiBtcCom.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BitcoinMainnetExplorerBlockApi::Mempool.into(),
                    height: None,
                },
                BlockInfo {
                    provider: BlockApi::BitcoinProvider(BitcoinProviderBlockApi::BitcoinCanister),
                    height: None,
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_testnet() {
        crate::storage::set_config(crate::config::Config::bitcoin_testnet());
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
                    provider: BlockApi::BitcoinProvider(BitcoinProviderBlockApi::BitcoinCanister),
                    height: None,
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_dogecoin_mainnet() {
        let staging_canister = false; // This parameter is not important. TODO(mducroux): revisit.
        crate::storage::set_config(crate::config::Config::dogecoin_mainnet(staging_canister));
        crate::test_utils::mock_all_outcalls_404();

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
                    provider: BlockApi::DogecoinProvider(DogecoinProviderBlockApi::DogecoinCanister),
                    height: None,
                },
                BlockInfo {
                    provider: DogecoinMainnetExplorerBlockApi::TokenView.into(),
                    height: None,
                },
            ]
        );
    }
}
