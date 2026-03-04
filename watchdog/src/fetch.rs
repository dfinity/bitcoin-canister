use crate::block_apis::{BitcoinBlockApi, BlockProvider};
use crate::storage;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// The data fetched from the external block APIs.
#[derive(Clone, Debug, Eq, PartialEq, CandidType, Serialize, Deserialize)]
pub struct BlockInfo {
    /// The provider of the block data.
    pub provider: String,

    /// The height of the block.
    pub height: Option<u64>,
}

impl BlockInfo {
    #[cfg(test)]
    pub fn new(provider: String, height: u64) -> Self {
        Self {
            provider,
            height: Some(height),
        }
    }
}

/// The data fetched from the external block APIs, Bitcoin only.
#[derive(Clone, Debug, Eq, PartialEq, CandidType, Serialize, Deserialize)]
pub struct LegacyBlockInfo {
    /// The provider of the Bitcoin block data.
    pub provider: BitcoinBlockApi,

    /// The height of the block.
    pub height: Option<u64>,
}

/// Error type for converting BlockInfo to LegacyBlockInfo.
pub struct BlockInfoConversionError {
    pub reason: String,
}

impl TryFrom<BlockInfo> for LegacyBlockInfo {
    type Error = BlockInfoConversionError;

    fn try_from(block_info: BlockInfo) -> Result<LegacyBlockInfo, Self::Error> {
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
            }
        };
        Ok(LegacyBlockInfo {
            provider,
            height: block_info.height,
        })
    }
}

/// Fetches the data from the external APIs and the canister.
pub async fn fetch_all_data() -> Vec<BlockInfo> {
    let canister = storage::get_canister();
    let config = storage::get_config();
    fetch_providers(config.get_providers(canister)).await
}

async fn fetch_providers(explorers: Vec<Box<dyn BlockProvider>>) -> Vec<BlockInfo> {
    let futures = explorers
        .iter()
        .map(|api| api.fetch_data())
        .collect::<Vec<_>>();
    let results = futures::future::join_all(futures).await;

    explorers
        .into_iter()
        .zip(results.into_iter())
        .map(|(provider, value)| BlockInfo {
            provider: provider.name(),
            height: value["height"].as_u64(),
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Canister;

    fn setup_canister(canister: Canister) {
        crate::storage::set_canister_config(canister);
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
                BlockInfo {
                    provider: "bitcoin_api_bitaps_com_mainnet".to_string(),
                    height: Some(700001),
                },
                BlockInfo {
                    provider: "bitcoin_api_blockchair_com_mainnet".to_string(),
                    height: Some(700002),
                },
                BlockInfo {
                    provider: "bitcoin_api_blockcypher_com_mainnet".to_string(),
                    height: Some(700003),
                },
                BlockInfo {
                    provider: "bitcoin_blockchain_info_mainnet".to_string(),
                    height: Some(700004),
                },
                BlockInfo {
                    provider: "bitcoin_blockstream_info_mainnet".to_string(),
                    height: Some(700005),
                },
                BlockInfo {
                    provider: "bitcoin_mempool_mainnet".to_string(),
                    height: Some(700008),
                },
                BlockInfo {
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
                BlockInfo {
                    provider: "bitcoin_mempool_testnet".to_string(),
                    height: Some(55002),
                },
                BlockInfo {
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
                BlockInfo {
                    provider: "dogecoin_api_blockchair_com_mainnet".to_string(),
                    height: Some(5926987),
                },
                BlockInfo {
                    provider: "dogecoin_api_blockcypher_com_mainnet".to_string(),
                    height: Some(5926989),
                },
                BlockInfo {
                    provider: "dogecoin_psy_protocol_mainnet".to_string(),
                    height: Some(5931072),
                },
                BlockInfo {
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
                BlockInfo {
                    provider: "bitcoin_api_bitaps_com_mainnet".to_string(),
                    height: None,
                },
                BlockInfo {
                    provider: "bitcoin_api_blockchair_com_mainnet".to_string(),
                    height: None,
                },
                BlockInfo {
                    provider: "bitcoin_api_blockcypher_com_mainnet".to_string(),
                    height: None,
                },
                BlockInfo {
                    provider: "bitcoin_blockchain_info_mainnet".to_string(),
                    height: None,
                },
                BlockInfo {
                    provider: "bitcoin_blockstream_info_mainnet".to_string(),
                    height: None,
                },
                BlockInfo {
                    provider: "bitcoin_mempool_mainnet".to_string(),
                    height: None,
                },
                BlockInfo {
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
                BlockInfo {
                    provider: "bitcoin_mempool_testnet".to_string(),
                    height: None,
                },
                BlockInfo {
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
                BlockInfo {
                    provider: "dogecoin_api_blockchair_com_mainnet".to_string(),
                    height: None,
                },
                BlockInfo {
                    provider: "dogecoin_api_blockcypher_com_mainnet".to_string(),
                    height: None,
                },
                BlockInfo {
                    provider: "dogecoin_psy_protocol_mainnet".to_string(),
                    height: None,
                },
                BlockInfo {
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
