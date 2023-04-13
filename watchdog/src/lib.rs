mod bitcoin_block_apis;
mod endpoints;
mod fetch;
mod health;
mod http;
mod storage;

#[cfg(test)]
mod test_utils;

use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::endpoints::*;
use crate::fetch::BlockInfo;
use crate::health::HealthStatus;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

/// The number of seconds to wait before the first tick.
const DELAY_BEFORE_FIRST_TICK_SEC: u64 = 1;

/// The number of seconds to wait between the ticks.
const INTERVAL_BETWEEN_TICKS_SEC: u64 = 60;

thread_local! {
    /// The local storage for the data fetched from the external APIs.
    static BLOCK_INFO_DATA: RwLock<HashMap<BitcoinBlockApi, BlockInfo>> = RwLock::new(HashMap::new());
}

/// This function is called when the canister is created.
#[ic_cdk_macros::init]
fn init() {
    ic_cdk_timers::set_timer(Duration::from_secs(DELAY_BEFORE_FIRST_TICK_SEC), || {
        ic_cdk::spawn(async {
            tick().await;
            ic_cdk_timers::set_timer_interval(
                Duration::from_secs(INTERVAL_BETWEEN_TICKS_SEC),
                || ic_cdk::spawn(tick()),
            );
        })
    });
}

/// This function is called after the canister is upgraded.
#[ic_cdk_macros::post_upgrade]
fn post_upgrade() {
    init()
}

/// Fetches the data from the external APIs and stores it in the local storage.
async fn tick() {
    let data = crate::fetch::fetch_all_data().await;
    data.into_iter().for_each(crate::storage::insert);
}

/// Returns the health status of the Bitcoin canister.
#[ic_cdk_macros::query]
fn health_status() -> HealthStatus {
    crate::health::calculate()
}

/// Prints a message to the console.
pub fn print(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    ic_cdk::api::print(msg);

    #[cfg(not(target_arch = "wasm32"))]
    println!("{}", msg);
}

// Exposing the endpoints in `lib.rs` (not in `main.rs`) to make them available
// to the downstream code which creates HTTP requests with transform functions.

#[ic_cdk_macros::query]
fn transform_api_bitaps_com_block(raw: TransformArgs) -> HttpResponse {
    endpoint_api_bitaps_com_block().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_api_blockchair_com_block(raw: TransformArgs) -> HttpResponse {
    endpoint_api_blockchair_com_block().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_api_blockcypher_com_block(raw: TransformArgs) -> HttpResponse {
    endpoint_api_blockcypher_com_block().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_bitcoin_canister(raw: TransformArgs) -> HttpResponse {
    endpoint_bitcoin_canister().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_blockchain_info_hash(raw: TransformArgs) -> HttpResponse {
    endpoint_blockchain_info_hash().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_blockchain_info_height(raw: TransformArgs) -> HttpResponse {
    endpoint_blockchain_info_height().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_blockstream_info_hash(raw: TransformArgs) -> HttpResponse {
    endpoint_blockstream_info_hash().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_blockstream_info_height(raw: TransformArgs) -> HttpResponse {
    endpoint_blockstream_info_height().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_chain_api_btc_com_block(raw: TransformArgs) -> HttpResponse {
    endpoint_chain_api_btc_com_block().transform(raw)
}
