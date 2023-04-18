mod bitcoin_block_apis;
mod config;
mod endpoints;
mod fetch;
mod health;
mod http;
mod storage;

#[cfg(test)]
mod test_utils;

use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::config::Config;
use crate::endpoints::*;
use crate::fetch::BlockInfo;
use crate::health::HealthStatus;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use ic_cdk_macros::{init, post_upgrade, query, update};

thread_local! {
    /// The local storage for the data fetched from the external APIs.
    static BLOCK_INFO_DATA: RwLock<HashMap<BitcoinBlockApi, BlockInfo>> = RwLock::new(HashMap::new());

    /// The local storage for the configuration.
    static CONFIG: RwLock<Config> = RwLock::new(Config::new());
}

/// This function is called when the canister is created.
#[init]
fn init() {
    ic_cdk_timers::set_timer(
        Duration::from_secs(crate::storage::get_config().delay_before_first_fetch_sec),
        || {
            ic_cdk::spawn(async {
                fetch_data().await;
                ic_cdk_timers::set_timer_interval(
                    Duration::from_secs(crate::storage::get_config().interval_between_fetches_sec),
                    || ic_cdk::spawn(fetch_data()),
                );
            })
        },
    );
}

/// This function is called after the canister is upgraded.
#[post_upgrade]
fn post_upgrade() {
    init()
}

/// Fetches the data from the external APIs and stores it in the local storage.
async fn fetch_data() {
    let data = crate::fetch::fetch_all_data().await;
    data.into_iter().for_each(crate::storage::insert);
}

/// Returns the health status of the Bitcoin canister.
#[query]
fn health_status() -> HealthStatus {
    crate::health::compare(
        crate::storage::get(&BitcoinBlockApi::BitcoinCanister),
        BitcoinBlockApi::explorers()
            .iter()
            .filter_map(crate::storage::get)
            .collect::<Vec<_>>(),
        crate::storage::get_config(),
    )
}

/// Returns the configuration of the watchdog canister.
#[query]
pub fn get_config() -> Config {
    crate::storage::get_config()
}

/// Sets the configuration of the watchdog canister.
#[update]
pub fn set_config(config: Config) {
    crate::storage::set_config(config)
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

#[query]
fn transform_api_bitaps_com_block(raw: TransformArgs) -> HttpResponse {
    endpoint_api_bitaps_com_block().transform(raw)
}

#[query]
fn transform_api_blockchair_com_block(raw: TransformArgs) -> HttpResponse {
    endpoint_api_blockchair_com_block().transform(raw)
}

#[query]
fn transform_api_blockcypher_com_block(raw: TransformArgs) -> HttpResponse {
    endpoint_api_blockcypher_com_block().transform(raw)
}

#[query]
fn transform_bitcoin_canister(raw: TransformArgs) -> HttpResponse {
    endpoint_bitcoin_canister().transform(raw)
}

#[query]
fn transform_blockchain_info_hash(raw: TransformArgs) -> HttpResponse {
    endpoint_blockchain_info_hash().transform(raw)
}

#[query]
fn transform_blockchain_info_height(raw: TransformArgs) -> HttpResponse {
    endpoint_blockchain_info_height().transform(raw)
}

#[query]
fn transform_blockstream_info_hash(raw: TransformArgs) -> HttpResponse {
    endpoint_blockstream_info_hash().transform(raw)
}

#[query]
fn transform_blockstream_info_height(raw: TransformArgs) -> HttpResponse {
    endpoint_blockstream_info_height().transform(raw)
}

#[query]
fn transform_chain_api_btc_com_block(raw: TransformArgs) -> HttpResponse {
    endpoint_chain_api_btc_com_block().transform(raw)
}
