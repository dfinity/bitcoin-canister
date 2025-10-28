mod api_access;
mod bitcoin_block_apis;
mod config;
mod endpoints;
mod fetch;
mod health;
mod http;
mod metrics;
mod storage;
mod types;

#[cfg(test)]
mod test_utils;

use crate::{
    bitcoin_block_apis::BitcoinBlockApi,
    config::{BitcoinNetwork, Config},
    endpoints::*,
    fetch::BlockInfo,
    health::HealthStatus,
    types::{CandidHttpRequest, CandidHttpResponse},
};
use ic_btc_interface::Flag;
use ic_cdk::management_canister::{HttpRequestResult, TransformArgs};
use ic_cdk_macros::{init, post_upgrade, query};
use ic_cdk_timers::TimerId;
use serde_bytes::ByteBuf;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

thread_local! {
    /// The local storage for the configuration.
    static CONFIG: RefCell<Config> = RefCell::new(Config::mainnet());

    /// The local storage for the data fetched from the external APIs.
    static BLOCK_INFO_DATA: RefCell<HashMap<BitcoinBlockApi, BlockInfo>> = RefCell::new(HashMap::new());

    /// The local storage for the API access target.
    static API_ACCESS_TARGET: RefCell<Option<Flag>> = const { RefCell::new(None) };
}

/// This function is called when the canister is created.
#[init]
fn init(network: BitcoinNetwork) {
    let config = match network {
        BitcoinNetwork::Mainnet => Config::mainnet(),
        BitcoinNetwork::Testnet => Config::testnet(),
    };
    crate::storage::set_config(config);

    set_timer(
        Duration::from_secs(crate::storage::get_config().delay_before_first_fetch_sec),
        || {
            ic_cdk::futures::spawn(async {
                tick().await;
                ic_cdk_timers::set_timer_interval(
                    Duration::from_secs(crate::storage::get_config().interval_between_fetches_sec),
                    || ic_cdk::futures::spawn(tick()),
                );
            })
        },
    );
}

/// This function is called after the canister is upgraded.
#[post_upgrade]
fn post_upgrade(network: BitcoinNetwork) {
    init(network)
}

/// Fetches the data from the external APIs and stores it in the local storage.
async fn fetch_block_info_data() {
    let bitcoin_network = crate::storage::get_config().bitcoin_network;
    let data = crate::fetch::fetch_all_data(bitcoin_network).await;
    data.into_iter().for_each(crate::storage::insert_block_info);
}

/// Periodically fetches data and sets the API access to the Bitcoin canister.
async fn tick() {
    fetch_block_info_data().await;
    crate::api_access::synchronise_api_access().await;
}

/// Returns the health status of the Bitcoin canister.
#[query]
fn health_status() -> HealthStatus {
    crate::health::health_status()
}

/// Returns the configuration of the watchdog canister.
#[query]
pub fn get_config() -> Config {
    crate::storage::get_config()
}

/// Returns the API access target for the Bitcoin canister.
#[query]
pub fn get_api_access_target() -> Option<Flag> {
    crate::storage::get_api_access_target()
}

/// Processes external HTTP requests.
#[query]
pub fn http_request(request: CandidHttpRequest) -> CandidHttpResponse {
    let parts: Vec<&str> = request.url.split('?').collect();
    match parts[0] {
        "/metrics" => crate::metrics::get_metrics(),
        _ => CandidHttpResponse {
            status_code: 404,
            headers: vec![],
            body: ByteBuf::from(String::from("Not found.")),
        },
    }
}

// Prints a message to the console.
fn print(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    ic_cdk::api::debug_print(msg);

    #[cfg(not(target_arch = "wasm32"))]
    println!("{}", msg);
}

#[allow(unused_variables)]
fn set_timer(delay: Duration, func: impl FnOnce() + 'static) -> TimerId {
    #[cfg(target_arch = "wasm32")]
    return ic_cdk_timers::set_timer(delay, func);

    #[cfg(not(target_arch = "wasm32"))]
    TimerId::default()
}

// Exposing the endpoints in `lib.rs` (not in `main.rs`) to make them available
// to the downstream code which creates HTTP requests with transform functions.

#[query]
fn transform_api_bitaps_com_block(raw: TransformArgs) -> HttpRequestResult {
    endpoint_api_bitaps_com_block_mainnet().transform(raw)
}

#[query]
fn transform_api_blockchair_com_block(raw: TransformArgs) -> HttpRequestResult {
    endpoint_api_blockchair_com_block_mainnet().transform(raw)
}

#[query]
fn transform_api_blockcypher_com_block(raw: TransformArgs) -> HttpRequestResult {
    endpoint_api_blockcypher_com_block_mainnet().transform(raw)
}

#[query]
fn transform_bitcoin_canister(raw: TransformArgs) -> HttpRequestResult {
    endpoint_bitcoin_canister().transform(raw)
}

#[query]
fn transform_bitcoinexplorer_org_block(raw: TransformArgs) -> HttpRequestResult {
    endpoint_bitcoinexplorer_org_block_mainnet().transform(raw)
}

#[query]
fn transform_blockchain_info_hash(raw: TransformArgs) -> HttpRequestResult {
    endpoint_blockchain_info_hash_mainnet().transform(raw)
}

#[query]
fn transform_blockchain_info_height(raw: TransformArgs) -> HttpRequestResult {
    endpoint_blockchain_info_height_mainnet().transform(raw)
}

#[query]
fn transform_blockstream_info_hash(raw: TransformArgs) -> HttpRequestResult {
    endpoint_blockstream_info_hash_mainnet().transform(raw)
}

#[query]
fn transform_blockstream_info_height(raw: TransformArgs) -> HttpRequestResult {
    endpoint_blockstream_info_height_mainnet().transform(raw)
}

#[query]
fn transform_chain_api_btc_com_block(raw: TransformArgs) -> HttpRequestResult {
    endpoint_chain_api_btc_com_block_mainnet().transform(raw)
}

#[query]
fn transform_mempool_height(raw: TransformArgs) -> HttpRequestResult {
    endpoint_mempool_height_mainnet().transform(raw)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn init_with_testnet_uses_testnet_config() {
        init(BitcoinNetwork::Testnet);
        assert_eq!(get_config(), Config::testnet());
    }

    #[test]
    fn init_with_mainnet_uses_mainnet_config() {
        init(BitcoinNetwork::Mainnet);
        assert_eq!(get_config(), Config::mainnet());
    }

    #[test]
    fn test_candid_interface_compatibility() {
        use candid_parser::utils::{service_compatible, CandidSource};
        use std::path::PathBuf;

        candid::export_service!();
        let rust_interface = __export_service();

        let candid_interface =
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("candid.did");

        service_compatible(
            CandidSource::Text(&rust_interface),
            CandidSource::File(candid_interface.as_path()),
        )
        .expect("The canister implementation is not compatible with the candid.did file");
    }
}
