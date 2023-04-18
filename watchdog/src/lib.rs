mod bitcoin_block_apis;
mod config;
mod endpoints;
mod fetch;
mod health;
mod http;
mod printer;
mod storage;

#[cfg(test)]
mod test_utils;

use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::endpoints::*;
use crate::fetch::BlockInfo;
use crate::health::HealthStatus;
use crate::printer::print;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use std::collections::HashMap;
use std::sync::Once;
use std::sync::RwLock;
use std::time::Duration;

thread_local! {
    /// The local storage for the data fetched from the external APIs.
    static BLOCK_INFO_DATA: RwLock<HashMap<BitcoinBlockApi, BlockInfo>> = RwLock::new(HashMap::new());
}

static START: Once = Once::new();

/// This function is called when the canister is created.
#[ic_cdk_macros::init]
fn init() {
    ic_cdk_timers::set_timer(
        Duration::from_secs(crate::config::DELAY_BEFORE_FIRST_FETCH_SEC),
        || {
            ic_cdk::spawn(async {
                fetch_data().await;
                ic_cdk_timers::set_timer_interval(
                    Duration::from_secs(crate::config::INTERVAL_BETWEEN_FETCHES_SEC),
                    || ic_cdk::spawn(fetch_data()),
                );
            })
        },
    );
}

/// This function is called after the canister is upgraded.
#[ic_cdk_macros::post_upgrade]
fn post_upgrade() {
    init()
}

/// Setup the stdlib hooks.
pub fn setup() {
    START.call_once(|| {
        printer::hook();
    });
}

/// Fetches the data from the external APIs and stores it in the local storage.
async fn fetch_data() {
    let data = crate::fetch::fetch_all_data().await;
    data.into_iter().for_each(crate::storage::insert);
}

/// Returns the health status of the Bitcoin canister.
#[ic_cdk_macros::query]
fn health_status() -> HealthStatus {
    crate::health::compare(
        crate::storage::get(&BitcoinBlockApi::BitcoinCanister),
        BitcoinBlockApi::explorers()
            .iter()
            .filter_map(crate::storage::get)
            .collect::<Vec<_>>(),
    )
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
