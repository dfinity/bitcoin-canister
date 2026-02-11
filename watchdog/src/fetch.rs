use crate::block_apis::{BitcoinBlockApi, BlockProvider};
use crate::{print, storage};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// The data fetched from the external block APIs.
#[derive(Clone, Debug, Eq, PartialEq, CandidType, Serialize, Deserialize)]
pub struct BlockInfo {
    /// The provider of the block data (canister and block API providers).
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

    let mut block_info = fetch_providers(config.get_providers(canister)).await;

    match fetch_canister_height().await {
        Some(height) => {
            let canister_block_info = BlockInfo {
                provider: canister.name(),
                height: Some(height),
            };
            block_info.push(canister_block_info);
        }
        None => {
            print("Error getting canister main chain height.");
            // Still add the canister with None height so health check can report it
            let canister_block_info = BlockInfo {
                provider: canister.name(),
                height: None,
            };
            block_info.push(canister_block_info);
        }
    };

    block_info
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

/// Fetches the canister main chain height.
#[cfg(target_arch = "wasm32")]
async fn fetch_canister_height() -> Option<u64> {
    let id = crate::storage::get_canister().canister_principal();
    let result = ic_cdk::call::Call::unbounded_wait(id, "get_main_chain_height")
        .with_args(&())
        .await
        .map_err(|err| {
            print(&format!(
                "Error getting canister main chain height: {:?}",
                err
            ))
        })
        .ok()?;
    let height = result
        .candid()
        .map_err(|err| {
            print(&format!(
                "Error decoding get_main_chain_height result: {:?}",
                err
            ))
        })
        .ok()?;
    Some(height)
}

/// Mock implementation for tests (non-wasm32 targets).
#[cfg(not(target_arch = "wasm32"))]
async fn fetch_canister_height() -> Option<u64> {
    MOCK_CANISTER_HEIGHT.with(|cell| *cell.borrow())
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static MOCK_CANISTER_HEIGHT: std::cell::RefCell<Option<u64>> =
        std::cell::RefCell::new(None);
}

/// Sets the mock response for `fetch_canister_height` in tests.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn mock_canister_height(height: Option<u64>) {
    MOCK_CANISTER_HEIGHT.with(|cell| {
        *cell.borrow_mut() = height;
    });
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Canister;
    use crate::test_utils;

    fn setup_canister(canister: Canister) {
        crate::storage::set_canister_config(canister);
    }

    #[tokio::test]
    async fn test_fetch_all_data_bitcoin_mainnet() {
        setup_canister(Canister::BitcoinMainnet);
        test_utils::mock_bitcoin_mainnet_outcalls();

        verify_bitcoin_mainnet_fetch_results().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_bitcoin_mainnet_staging() {
        setup_canister(Canister::BitcoinMainnetStaging);
        test_utils::mock_bitcoin_mainnet_outcalls();

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
                    height: Some(test_utils::BITCOIN_MAINNET_CANISTER_HEIGHT),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_testnet() {
        setup_canister(Canister::BitcoinTestnet);
        test_utils::mock_bitcoin_testnet_outcalls();

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
                    height: Some(test_utils::BITCOIN_TESTNET_CANISTER_HEIGHT),
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
                    provider: "dogecoin_tokenview_mainnet".to_string(),
                    height: Some(5931072),
                },
                BlockInfo {
                    provider: "dogecoin_canister".to_string(),
                    height: Some(test_utils::DOGECOIN_MAINNET_CANISTER_HEIGHT),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_dogecoin_mainnet() {
        setup_canister(Canister::DogecoinMainnet);
        test_utils::mock_dogecoin_mainnet_outcalls();

        verify_dogecoin_mainnet_fetch_results().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_dogecoin_mainnet_staging() {
        setup_canister(Canister::DogecoinMainnetStaging);
        test_utils::mock_dogecoin_mainnet_outcalls();

        verify_dogecoin_mainnet_fetch_results().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_mainnet() {
        setup_canister(Canister::BitcoinMainnet);
        test_utils::mock_all_outcalls_404();

        verify_bitcoin_mainnet_fetch_failed_404().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_mainnet_staging() {
        setup_canister(Canister::BitcoinMainnetStaging);
        test_utils::mock_all_outcalls_404();

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
        test_utils::mock_all_outcalls_404();

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
                    provider: "dogecoin_tokenview_mainnet".to_string(),
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
        test_utils::mock_all_outcalls_404();

        verify_dogecoin_mainnet_fetch_failed_404().await;
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_dogecoin_mainnet_staging() {
        setup_canister(Canister::DogecoinMainnetStaging);
        test_utils::mock_all_outcalls_404();

        verify_dogecoin_mainnet_fetch_failed_404().await;
    }

    #[tokio::test]
    async fn test_bitcoin_canister_mainnet() {
        // Bitcoin canister uses inter-canister calls, not HTTP requests
        test_utils::mock_bitcoin_mainnet_outcalls();
        let height = fetch_canister_height().await;
        assert_eq!(height, Some(test_utils::BITCOIN_MAINNET_CANISTER_HEIGHT));
    }

    #[tokio::test]
    async fn test_bitcoin_canister_testnet() {
        // Bitcoin canister uses inter-canister calls, not HTTP requests
        test_utils::mock_bitcoin_testnet_outcalls();
        let height = fetch_canister_height().await;
        assert_eq!(height, Some(test_utils::BITCOIN_TESTNET_CANISTER_HEIGHT));
    }

    #[tokio::test]
    async fn test_dogecoin_canister_mainnet() {
        test_utils::mock_dogecoin_mainnet_outcalls();
        let height = fetch_canister_height().await;
        assert_eq!(height, Some(test_utils::DOGECOIN_MAINNET_CANISTER_HEIGHT));
    }

    #[tokio::test]
    async fn test_fetch_canister_height_failed() {
        test_utils::mock_all_outcalls_404();
        let height = fetch_canister_height().await;
        assert_eq!(height, None);
    }
}
