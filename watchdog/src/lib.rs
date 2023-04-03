mod info;
mod remote_api;
mod time;
mod types;

// #[cfg(not(target_arch = "wasm32"))]
// mod ic_http_mock;

pub use crate::info::{Config, Info};

use futures::future::{join_all, BoxFuture};
use remote_api::{
    ApiBlockchairCom, ApiBlockcypherCom, BitcoinCanister, BlockchainInfo, BlockstreamInfo,
    ChainApiBtcCom,
};

#[cfg(target_arch = "wasm32")]
pub fn print(msg: &str) {
    ic_cdk::api::print(msg);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn print(msg: &str) {
    println!("{}", msg);
}

/// Fetches the latest block height from the remote APIs.
pub async fn tick_async() {
    print("tick_async...");

    let futures: Vec<BoxFuture<_>> = vec![
        //Box::pin(ApiBitapsCom::fetch()),  // TODO: investigate why it lags behind.
        Box::pin(ApiBlockchairCom::fetch()),
        Box::pin(ApiBlockcypherCom::fetch()),
        Box::pin(BitcoinCanister::fetch()),
        Box::pin(BlockchainInfo::fetch()),
        Box::pin(BlockstreamInfo::fetch()),
        Box::pin(ChainApiBtcCom::fetch()),
    ];
    join_all(futures).await;
}

/// Returns the health info report based on the latest block heights.
pub fn get_info() -> Info {
    let mut heights = vec![];
    // if let Some(height) = ApiBitapsCom::get_height() {
    //    heights.push((ApiBitapsCom::host(), height));
    // }
    if let Some(height) = ApiBlockchairCom::get_height() {
        heights.push((ApiBlockchairCom::host(), height));
    }
    if let Some(height) = ApiBlockcypherCom::get_height() {
        heights.push((ApiBlockcypherCom::host(), height));
    }
    if let Some(height) = BlockchainInfo::get_height() {
        heights.push((BlockchainInfo::host(), height));
    }
    if let Some(height) = BlockstreamInfo::get_height() {
        heights.push((BlockstreamInfo::host(), height));
    }
    if let Some(height) = ChainApiBtcCom::get_height() {
        heights.push((ChainApiBtcCom::host(), height));
    }
    let heights = heights
        .into_iter()
        .map(|(n, h)| (n.to_string(), h))
        .collect();

    Info::new(Config::default(), BitcoinCanister::get_height(), heights)
}
