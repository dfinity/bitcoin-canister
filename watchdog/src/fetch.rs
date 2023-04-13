use crate::bitcoin_block_apis::BitcoinBlockApi;

#[derive(Clone, Debug)]
pub struct BlockInfo {
    pub provider: BitcoinBlockApi,
    pub height: Option<u64>,
    hash: Option<String>,
    previous_hash: Option<String>,
}

impl BlockInfo {
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
