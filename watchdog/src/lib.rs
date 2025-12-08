mod api_access;
mod block_apis;
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

use crate::block_apis::BlockApi;
use crate::config::Network;
use crate::fetch::BlockInfoInternal;
use crate::health::HealthStatusV2;
use crate::{
    config::Config,
    endpoints::*,
    health::HealthStatus,
    types::WatchdogArg,
    types::{CandidHttpRequest, CandidHttpResponse},
};
use ic_btc_interface::Flag;
use ic_cdk::{
    init,
    management_canister::{HttpRequestResult, TransformArgs},
    post_upgrade, query,
};
use ic_cdk_timers::TimerId;
use serde_bytes::ByteBuf;
use std::{cell::RefCell, collections::HashMap, future::Future, time::Duration};

thread_local! {
    /// The local storage for the data fetched from the external APIs.
    static BLOCK_INFO_DATA: RefCell<HashMap<BlockApi, BlockInfoInternal>> = RefCell::new(HashMap::new());

    /// The local storage for the API access target.
    static API_ACCESS_TARGET: RefCell<Option<Flag>> = const { RefCell::new(None) };

    /// Counter for health_status endpoint calls.
    static HEALTH_STATUS_CALLS: RefCell<u64> = const { RefCell::new(0) };
}

/// This function is called when the canister is created.
#[init]
fn init(watchdog_arg: WatchdogArg) {
    let target = match watchdog_arg {
        WatchdogArg::Init(args) => args.target,
        WatchdogArg::Upgrade(_) => panic!("cannot initialize canister during upgrade"),
    };

    let config = Config::for_target(target);
    storage::set_config(config);

    start_block_info_fetch_loop();
}

/// This function is called after the canister is upgraded.
#[post_upgrade]
fn post_upgrade(watchdog_arg: Option<WatchdogArg>) {
    if let Some(WatchdogArg::Init(_)) = watchdog_arg {
        panic!("cannot upgrade canister with init args");
    };
    start_block_info_fetch_loop();
}

fn start_block_info_fetch_loop() {
    set_timer(
        Duration::from_secs(storage::get_config().delay_before_first_fetch_sec),
        async {
            tick().await;
            ic_cdk_timers::set_timer_interval(
                Duration::from_secs(storage::get_config().interval_between_fetches_sec),
                || async { tick().await },
            );
        },
    );
}

/// Fetches the data from the external APIs and stores it in the local storage.
async fn fetch_block_info_data() {
    let network = crate::storage::get_config().network;
    let data = crate::fetch::fetch_all_data(network).await;
    data.into_iter().for_each(crate::storage::insert_block_info);
}

/// Periodically fetches data and sets the API access to the canister monitored.
async fn tick() {
    fetch_block_info_data().await;
    crate::api_access::synchronise_api_access().await;
}

/// Returns the health status of the canister monitored (for Bitcoin only).
#[query]
fn health_status() -> HealthStatus {
    storage::increment_health_status_calls();
    let network = storage::get_config().network;
    match network {
        Network::BitcoinMainnet | Network::BitcoinTestnet => {
            HealthStatus::from(health::health_status_internal())
        }
        _ => panic!("health_status can only be called for Bitcoin networks"),
    }
}

/// Returns the health status of the canister monitored.
#[query]
fn health_status_v2() -> HealthStatusV2 {
    health::health_status_internal().into()
}

/// Returns the configuration of the watchdog canister.
#[query]
pub fn get_config() -> Config {
    crate::storage::get_config()
}

/// Returns the API access target for the canister monitored.
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
fn set_timer(delay: Duration, future: impl Future<Output = ()> + 'static) -> TimerId {
    #[cfg(target_arch = "wasm32")]
    return ic_cdk_timers::set_timer(delay, future);

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
fn transform_blockchain_info_hash(raw: TransformArgs) -> HttpRequestResult {
    endpoint_blockchain_info_hash_mainnet().transform(raw)
}

#[query]
fn transform_blockchain_info_height(raw: TransformArgs) -> HttpRequestResult {
    endpoint_blockchain_info_height_mainnet().transform(raw)
}

#[query]
fn transform_blockexplorer_one_block(raw: TransformArgs) -> HttpRequestResult {
    endpoint_blockexplorer_one_block_mainnet().transform(raw)
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
fn transform_dogecoin_api_blockchair_com_block(raw: TransformArgs) -> HttpRequestResult {
    endpoint_dogecoin_api_blockchair_com_block_mainnet().transform(raw)
}

#[query]
fn transform_dogecoin_api_blockcypher_com_block(raw: TransformArgs) -> HttpRequestResult {
    endpoint_dogecoin_api_blockcypher_com_block_mainnet().transform(raw)
}

#[query]
fn transform_dogecoin_tokenview_height(raw: TransformArgs) -> HttpRequestResult {
    endpoint_dogecoin_tokenview_height_mainnet().transform(raw)
}

#[query]
fn transform_dogecoin_canister(raw: TransformArgs) -> HttpRequestResult {
    endpoint_dogecoin_canister().transform(raw)
}

#[query]
fn transform_mempool_height(raw: TransformArgs) -> HttpRequestResult {
    endpoint_mempool_height_mainnet().transform(raw)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Canister;
    use crate::types::InitArg;

    #[test]
    fn init_with_bitcoin_testnet_uses_testnet_config() {
        let canister = Canister::BitcoinTestnet;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);
        assert_eq!(get_config(), Config::for_target(Canister::BitcoinTestnet));
    }

    #[test]
    fn init_with_bitcoin_mainnet_uses_mainnet_config() {
        let canister = Canister::BitcoinMainnet;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);
        assert_eq!(get_config(), Config::for_target(Canister::BitcoinMainnet));
    }

    #[test]
    fn init_with_bitcoin_mainnet_staging_uses_mainnet_staging_config() {
        let canister = Canister::BitcoinMainnetStaging;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);
        assert_eq!(
            get_config(),
            Config::for_target(Canister::BitcoinMainnetStaging)
        );
    }

    #[test]
    fn init_with_dogecoin_mainnet_uses_mainnet_config() {
        let canister = Canister::DogecoinMainnet;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);
        assert_eq!(get_config(), Config::for_target(Canister::DogecoinMainnet));
    }

    #[test]
    fn init_with_dogecoin_mainnet_staging_uses_mainnet_staging_config() {
        let canister = Canister::DogecoinMainnetStaging;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);
        assert_eq!(
            get_config(),
            Config::for_target(Canister::DogecoinMainnetStaging)
        );
    }

    #[test]
    fn test_post_upgrade_with_no_args() {
        let canister = Canister::DogecoinMainnet;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);

        let config_before = get_config();

        post_upgrade(None);

        let config_after = get_config();

        assert_eq!(config_before, config_after);
        assert_eq!(config_after, Config::for_target(Canister::DogecoinMainnet));
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
