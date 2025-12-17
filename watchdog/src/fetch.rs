use crate::block_apis::BitcoinBlockApi;
use crate::storage;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// The data fetched from the external block APIs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockInfoInternal {
    /// The provider of the block data.
    pub provider: String,

    /// The height of the block.
    pub height: Option<u64>,
}

impl BlockInfoInternal {
    #[cfg(test)]
    pub fn new(provider: String, height: u64) -> Self {
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
        let provider = match block_info.provider.as_str() {
            "bitcoin_canister" => BitcoinBlockApi::BitcoinCanister,
            "bitcoin_api_bitaps_com_mainnet" => BitcoinBlockApi::ApiBitapsComMainnet,
            "bitcoin_api_blockchair_com_mainnet" => BitcoinBlockApi::ApiBlockchairComMainnet,
            "bitcoin_api_blockcypher_com_mainnet" => BitcoinBlockApi::ApiBlockcypherComMainnet,
            "bitcoin_blockchain_info_mainnet" => BitcoinBlockApi::BlockchainInfoMainnet,
            "bitcoin_blockstream_info_mainnet" => BitcoinBlockApi::BlockstreamInfoMainnet,
            "bitcoin_mempool_mainnet" => BitcoinBlockApi::MempoolMainnet,
            "bitcoin_mempool_testnet" => BitcoinBlockApi::MempoolTestnet,
            _ => {
                return Err(BlockInfoConversionError {
                    reason: "BlockInfo can only contain Bitcoin providers".to_string(),
                });
            } // TODO: add bitcoin_canister testnet
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

/// Fetches the data from the external APIs and the canister.
pub async fn fetch_all_data() -> Vec<BlockInfoInternal> {
    let canister = storage::get_canister();
    let providers = canister.all_providers();

    let futures = providers
        .iter()
        .map(|api| api.fetch_data())
        .collect::<Vec<_>>();
    let results = futures::future::join_all(futures).await;

    providers
        .into_iter()
        .zip(results.into_iter())
        .map(|(provider, value)| BlockInfoInternal {
            provider: provider.to_string(),
            height: value["height"].as_u64(),
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Canister;

    fn setup_canister(canister: Canister) {
        crate::storage::set_canister(canister);
        crate::storage::set_config(crate::config::StoredConfig::for_target(canister));
    }

    #[tokio::test]
    async fn test_fetch_all_data_bitcoin_mainnet() {
        setup_canister(Canister::BitcoinMainnet);
        crate::test_utils::mock_bitcoin_mainnet_outcalls();

        verify_bitcoin_mainnet_fetch_results().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_bitcoin_mainnet_staging() {
        setup_canister(Canister::BitcoinMainnetStaging);
        crate::test_utils::mock_bitcoin_mainnet_outcalls();

        verify_bitcoin_mainnet_fetch_results().await;
    }

    async fn verify_bitcoin_mainnet_fetch_results() {
        let result = fetch_all_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfoInternal {
                    provider: "bitcoin_api_bitaps_com_mainnet".to_string(),
                    height: Some(700001),
                },
                BlockInfoInternal {
                    provider: "bitcoin_api_blockchair_com_mainnet".to_string(),
                    height: Some(700002),
                },
                BlockInfoInternal {
                    provider: "bitcoin_api_blockcypher_com_mainnet".to_string(),
                    height: Some(700003),
                },
                BlockInfoInternal {
                    provider: "bitcoin_blockchain_info_mainnet".to_string(),
                    height: Some(700004),
                },
                BlockInfoInternal {
                    provider: "bitcoin_blockstream_info_mainnet".to_string(),
                    height: Some(700005),
                },
                BlockInfoInternal {
                    provider: "bitcoin_mempool_mainnet".to_string(),
                    height: Some(700008),
                },
                BlockInfoInternal {
                    provider: "bitcoin_canister".to_string(),
                    height: Some(700007),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_testnet() {
        setup_canister(Canister::BitcoinTestnet);
        crate::test_utils::mock_bitcoin_testnet_outcalls();

        let result = fetch_all_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfoInternal {
                    provider: "bitcoin_mempool_testnet".to_string(),
                    height: Some(55002),
                },
                BlockInfoInternal {
                    provider: "bitcoin_canister".to_string(),
                    height: Some(55001),
                },
            ]
        );
    }

    async fn verify_dogecoin_mainnet_fetch_results() {
        let result = fetch_all_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfoInternal {
                    provider: "dogecoin_api_blockchair_com_mainnet".to_string(),
                    height: Some(5926987),
                },
                BlockInfoInternal {
                    provider: "dogecoin_api_blockcypher_com_mainnet".to_string(),
                    height: Some(5926989),
                },
                BlockInfoInternal {
                    provider: "dogecoin_tokenview_mainnet".to_string(),
                    height: Some(5931072),
                },
                BlockInfoInternal {
                    provider: "dogecoin_canister".to_string(),
                    height: Some(5931098),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_dogecoin_mainnet() {
        setup_canister(Canister::DogecoinMainnet);
        crate::test_utils::mock_dogecoin_mainnet_outcalls();

        verify_dogecoin_mainnet_fetch_results().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_dogecoin_mainnet_staging() {
        setup_canister(Canister::DogecoinMainnetStaging);
        crate::test_utils::mock_dogecoin_mainnet_outcalls();

        verify_dogecoin_mainnet_fetch_results().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_mainnet() {
        setup_canister(Canister::BitcoinMainnet);
        crate::test_utils::mock_all_outcalls_404();

        verify_bitcoin_mainnet_fetch_failed_404().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_mainnet_staging() {
        setup_canister(Canister::BitcoinMainnetStaging);
        crate::test_utils::mock_all_outcalls_404();

        verify_bitcoin_mainnet_fetch_failed_404().await;
    }

    async fn verify_bitcoin_mainnet_fetch_failed_404() {
        let result = fetch_all_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfoInternal {
                    provider: "bitcoin_api_bitaps_com_mainnet".to_string(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: "bitcoin_api_blockchair_com_mainnet".to_string(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: "bitcoin_api_blockcypher_com_mainnet".to_string(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: "bitcoin_blockchain_info_mainnet".to_string(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: "bitcoin_blockstream_info_mainnet".to_string(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: "bitcoin_mempool_mainnet".to_string(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: "bitcoin_canister".to_string(),
                    height: None,
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_testnet() {
        setup_canister(Canister::BitcoinTestnet);
        crate::test_utils::mock_all_outcalls_404();

        let result = fetch_all_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfoInternal {
                    provider: "bitcoin_mempool_testnet".to_string(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: "bitcoin_canister".to_string(),
                    height: None,
                },
            ]
        );
    }

    async fn verify_dogecoin_mainnet_fetch_failed_404() {
        let result = fetch_all_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfoInternal {
                    provider: "dogecoin_api_blockchair_com_mainnet".to_string(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: "dogecoin_api_blockcypher_com_mainnet".to_string(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: "dogecoin_tokenview_mainnet".to_string(),
                    height: None,
                },
                BlockInfoInternal {
                    provider: "dogecoin_canister".to_string(),
                    height: None,
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_dogecoin_mainnet() {
        setup_canister(Canister::DogecoinMainnet);
        crate::test_utils::mock_all_outcalls_404();

        verify_dogecoin_mainnet_fetch_failed_404().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_dogecoin_mainnet_staging() {
        setup_canister(Canister::DogecoinMainnetStaging);
        crate::test_utils::mock_all_outcalls_404();

        verify_dogecoin_mainnet_fetch_failed_404().await;
    }
}
