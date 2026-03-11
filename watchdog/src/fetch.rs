use crate::block_apis::BitcoinBlockApi;
#[cfg(target_arch = "wasm32")]
use crate::print;
use crate::storage;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// The data fetched from the external block APIs.
#[derive(Clone, Debug, Eq, PartialEq, CandidType, Serialize, Deserialize)]
pub struct BlockInfo {
    /// The API provider of the block data.
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

impl TryFrom<BlockInfo> for LegacyBlockInfo {
    type Error = String;

    fn try_from(block_info: BlockInfo) -> Result<LegacyBlockInfo, Self::Error> {
        let provider = match block_info.provider.as_str() {
            "bitcoin_canister" => BitcoinBlockApi::BitcoinCanister,
            "bitcoin_api_blockchair_com_mainnet" => BitcoinBlockApi::ApiBlockchairComMainnet,
            "bitcoin_api_blockcypher_com_mainnet" => BitcoinBlockApi::ApiBlockcypherComMainnet,
            "bitcoin_blockchain_info_mainnet" => BitcoinBlockApi::BlockchainInfoMainnet,
            "bitcoin_blockstream_info_mainnet" => BitcoinBlockApi::BlockstreamInfoMainnet,
            "bitcoin_mempool_mainnet" => BitcoinBlockApi::MempoolMainnet,
            "bitcoin_mempool_testnet" => BitcoinBlockApi::MempoolTestnet,
            _ => {
                return Err(format!("unknown Bitcoin provider: {}", block_info.provider));
            }
        };
        Ok(LegacyBlockInfo {
            provider,
            height: block_info.height,
        })
    }
}

/// Fetches block info from the block provider APIs.
pub async fn fetch_all_providers_data() -> Vec<BlockInfo> {
    let config = storage::get_config();
    let providers = config.get_providers();
    let futures = providers
        .iter()
        .map(|api| api.fetch_data())
        .collect::<Vec<_>>();
    let results = futures::future::join_all(futures).await;

    providers
        .into_iter()
        .zip(results.into_iter())
        .map(|(provider, value)| BlockInfo {
            provider: provider.name(),
            height: value["height"].as_u64(),
        })
        .collect()
}

/// Fetches the canister main chain height via the `get_blockchain_info` endpoint.
#[cfg(target_arch = "wasm32")]
pub async fn fetch_canister_height() -> Option<u64> {
    let id = storage::get_config().canister_principal;
    let result = ic_cdk::call::Call::unbounded_wait(id, "get_blockchain_info")
        .with_args(&())
        .await
        .map_err(|err| print(&format!("Error calling get_blockchain_info: {:?}", err)))
        .ok()?;
    let info: ic_btc_interface::BlockchainInfo = result
        .candid()
        .map_err(|err| {
            print(&format!(
                "Error decoding get_blockchain_info result: {:?}",
                err
            ))
        })
        .ok()?;
    Some(info.height as u64)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_canister_height() -> Option<u64> {
    None
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
        let result = fetch_all_providers_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: "bitcoin_api_bitcore_io_mainnet".to_string(),
                    height: Some(700009),
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
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_testnet() {
        setup_canister(Canister::BitcoinTestnet);
        test_utils::mock_bitcoin_testnet_outcalls();

        let result = fetch_all_providers_data().await;
        assert_eq!(
            result,
            vec![BlockInfo {
                provider: "bitcoin_mempool_testnet".to_string(),
                height: Some(55002),
            }]
        );
    }

    async fn verify_dogecoin_mainnet_fetch_results() {
        let result = fetch_all_providers_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: "dogecoin_api_bitcore_io_mainnet".to_string(),
                    height: Some(5931100),
                },
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
        let result = fetch_all_providers_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: "bitcoin_api_bitcore_io_mainnet".to_string(),
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
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_all_data_failed_404_bitcoin_testnet() {
        setup_canister(Canister::BitcoinTestnet);
        test_utils::mock_all_outcalls_404();

        let result = fetch_all_providers_data().await;
        assert_eq!(
            result,
            vec![BlockInfo {
                provider: "bitcoin_mempool_testnet".to_string(),
                height: None,
            }]
        );
    }

    async fn verify_dogecoin_mainnet_fetch_failed_404() {
        let result = fetch_all_providers_data().await;
        assert_eq!(
            result,
            vec![
                BlockInfo {
                    provider: "dogecoin_api_bitcore_io_mainnet".to_string(),
                    height: None,
                },
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
}
