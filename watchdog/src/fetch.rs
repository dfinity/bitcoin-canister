use crate::block_apis::{
    BitcoinBlockApi, BitcoinMainnetExplorerBlockApi, BitcoinMainnetProviderBlockApi,
    BitcoinTestnetExplorerBlockApi, BitcoinTestnetProviderBlockApi, BlockApi, BlockApiTrait,
    DogecoinProviderBlockApi,
};
use crate::config::Network;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// The data fetched from the external block APIs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockInfoInternal {
    /// The provider of the block data.
    pub provider: BlockApi,

    /// The height of the block.
    pub height: Option<u64>,
}

impl BlockInfoInternal {
    #[cfg(test)]
    pub fn new(provider: BlockApi, height: u64) -> Self {
        Self {
            provider,
            height: Some(height),
        }
    }
}

/// The data fetched from the external Bitcoin block APIs.
#[derive(Clone, Debug, Eq, PartialEq, CandidType, Serialize, Deserialize)]
pub struct BlockInfo {
    /// The provider of the Bitcoin block data.
    pub provider: BitcoinBlockApi,

    /// The height of the block.
    pub height: Option<u64>,
}

/// Error type for converting BlockInfoInternal to BlockInfo.
pub struct BlockInfoConversionError {
    pub reason: String,
}

impl TryFrom<BlockInfoInternal> for BlockInfo {
    type Error = BlockInfoConversionError;

    fn try_from(block_info: BlockInfoInternal) -> Result<BlockInfo, Self::Error> {
        let provider = match block_info.provider {
            BlockApi::BitcoinMainnetProvider(provider) => match provider {
                BitcoinMainnetProviderBlockApi::BitcoinCanister => BitcoinBlockApi::BitcoinCanister,
                BitcoinMainnetProviderBlockApi::Mainnet(explorer) => match explorer {
                    BitcoinMainnetExplorerBlockApi::ApiBitapsCom => {
                        BitcoinBlockApi::ApiBitapsComMainnet
                    }
                    BitcoinMainnetExplorerBlockApi::ApiBlockchairCom => {
                        BitcoinBlockApi::ApiBlockchairComMainnet
                    }
                    BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom => {
                        BitcoinBlockApi::ApiBlockcypherComMainnet
                    }
                    BitcoinMainnetExplorerBlockApi::BlockchainInfo => {
                        BitcoinBlockApi::BlockchainInfoMainnet
                    }
                    BitcoinMainnetExplorerBlockApi::BlockstreamInfo => {
                        BitcoinBlockApi::BlockstreamInfoMainnet
                    }
                    BitcoinMainnetExplorerBlockApi::Mempool => BitcoinBlockApi::MempoolMainnet,
                },
            },
            BlockApi::BitcoinTestnetProvider(provider) => match provider {
                BitcoinTestnetProviderBlockApi::BitcoinCanister => BitcoinBlockApi::BitcoinCanister,
                BitcoinTestnetProviderBlockApi::Testnet(explorer) => match explorer {
                    BitcoinTestnetExplorerBlockApi::Mempool => BitcoinBlockApi::MempoolTestnet,
                },
            },
            BlockApi::DogecoinProvider(_) => {
                return Err(BlockInfoConversionError {
                    reason: "BlockInfo can only contain Bitcoin providers".to_string(),
                });
            }
        };
        Ok(BlockInfo {
            provider,
            height: block_info.height,
        })
    }
}

/// The data fetched from the external block APIs.
#[derive(Clone, Debug, Eq, PartialEq, CandidType, Serialize, Deserialize)]
pub struct BlockInfoV2 {
    /// The provider of the block data.
    pub provider: String,

    /// The height of the block.
    pub height: Option<u64>,
}

impl From<BlockInfoInternal> for BlockInfoV2 {
    fn from(block_info: BlockInfoInternal) -> BlockInfoV2 {
        BlockInfoV2 {
            provider: block_info.provider.to_string(),
            height: block_info.height,
        }
    }
}

pub async fn fetch_all_data(network: Network) -> Vec<BlockInfoInternal> {
    match network {
        Network::BitcoinMainnet => {
            fetch_all_data_for_providers::<BitcoinMainnetProviderBlockApi>().await
        }
        Network::BitcoinTestnet => {
            fetch_all_data_for_providers::<BitcoinTestnetProviderBlockApi>().await
        }
        Network::DogecoinMainnet => {
            fetch_all_data_for_providers::<DogecoinProviderBlockApi>().await
        }
    }
}

/// Fetches the data from the external APIs.
pub async fn fetch_all_data_for_providers<P: BlockApiTrait + Into<BlockApi>>() -> Vec<BlockInfoInternal> {
    let api_providers = P::network_providers();

    let futures = api_providers
        .iter()
        .map(|api| api.fetch_data())
        .collect::<Vec<_>>();
    let results = futures::future::join_all(futures).await;

    let result: Vec<_> = api_providers
        .iter()
        .zip(results.iter())
        .map(|(api, value)| BlockInfoInternal {
            provider: api.clone().into(),
            height: value["height"].as_u64(),
        })
        .collect();

    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block_apis::{
        BitcoinMainnetExplorerBlockApi, BitcoinTestnetExplorerBlockApi,
        DogecoinMainnetExplorerBlockApi, DogecoinProviderBlockApi,
    };
    use crate::config::Canister;

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
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBitapsCom.into(),
                    height: Some(700001),
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    height: Some(700002),
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    height: Some(700003),
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                    height: Some(700004),
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                    height: Some(700005),
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::Mempool.into(),
                    height: Some(700008),
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetProviderBlockApi::BitcoinCanister.into(),
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
                BlockInfoInternal {
                    provider: BitcoinTestnetExplorerBlockApi::Mempool.into(),
                    height: Some(55002),
                },
                BlockInfoInternal {
                    provider: BitcoinTestnetProviderBlockApi::BitcoinCanister.into(),
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
                BlockInfoInternal {
                    provider: DogecoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    height: Some(5926987),
                },
                BlockInfoInternal {
                    provider: DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    height: Some(5926989),
                },
                BlockInfoInternal {
                    provider: DogecoinMainnetExplorerBlockApi::TokenView.into(),
                    height: Some(5931072),
                },
                BlockInfoInternal {
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
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBitapsCom.into(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::BlockchainInfo.into(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::BlockstreamInfo.into(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetExplorerBlockApi::Mempool.into(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: BitcoinMainnetProviderBlockApi::BitcoinCanister.into(),
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
                BlockInfoInternal {
                    provider: BitcoinTestnetExplorerBlockApi::Mempool.into(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: BitcoinTestnetProviderBlockApi::BitcoinCanister.into(),
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
                BlockInfoInternal {
                    provider: DogecoinMainnetExplorerBlockApi::ApiBlockchairCom.into(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: DogecoinMainnetExplorerBlockApi::ApiBlockcypherCom.into(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: DogecoinMainnetExplorerBlockApi::TokenView.into(),
                    height: None,
                },
                BlockInfoInternal {
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
