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

use crate::config::{Config, Network};
use crate::fetch::BlockInfo;
use crate::health::HealthStatus;
use crate::{
    endpoints::*,
    health::LegacyHealthStatus,
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
use std::convert::TryFrom;
use std::{cell::RefCell, collections::HashMap, future::Future, time::Duration};

thread_local! {
    /// The local storage for the data fetched from the external APIs.
    static BLOCK_INFO_DATA: RefCell<HashMap<String, BlockInfo>> = RefCell::new(HashMap::new());

    /// The local storage for the API access target.
    static API_ACCESS_TARGET: RefCell<Option<Flag>> = const { RefCell::new(None) };
}

/// This function is called when the canister is created.
#[init]
fn init(watchdog_arg: WatchdogArg) {
    let target = match watchdog_arg {
        WatchdogArg::Init(args) => args.target,
        WatchdogArg::Upgrade(_) => panic!("cannot initialize canister during upgrade"),
    };

    storage::set_canister_config(target);

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
    let data = crate::fetch::fetch_all_data().await;
    data.into_iter().for_each(crate::storage::insert_block_info);
}

/// Periodically fetches data and sets the API access to the canister monitored.
async fn tick() {
    fetch_block_info_data().await;
    crate::api_access::synchronise_api_access().await;
}

/// Returns the health status of the canister monitored (for Bitcoin only).
#[query]
fn health_status() -> LegacyHealthStatus {
    let network = storage::get_canister().network();
    match network {
        Network::BitcoinMainnet | Network::BitcoinTestnet => {
            LegacyHealthStatus::try_from(health::health_status()).unwrap_or_else(|e| {
                panic!(
                    "Failed to convert health status for Bitcoin network: {}",
                    e.reason
                )
            })
        }
        _ => panic!("health_status can only be called for Bitcoin networks"),
    }
}

/// Returns the health status of the canister monitored.
#[query]
fn health_status_v2() -> HealthStatus {
    health::health_status()
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
fn transform_bitcoin_canister(raw: TransformArgs) -> HttpRequestResult {
    endpoint_bitcoin_canister().transform(raw)
}

#[query]
fn transform_bitcoin_mainnet_api_bitaps_com(raw: TransformArgs) -> HttpRequestResult {
    endpoint_bitcoin_mainnet_api_bitaps_com().transform(raw)
}

#[query]
fn transform_bitcoin_mainnet_api_blockchair_com(raw: TransformArgs) -> HttpRequestResult {
    endpoint_bitcoin_mainnet_api_blockchair_com().transform(raw)
}

#[query]
fn transform_bitcoin_mainnet_api_blockcypher_com(raw: TransformArgs) -> HttpRequestResult {
    endpoint_bitcoin_mainnet_api_blockcypher_com().transform(raw)
}

#[query]
fn transform_bitcoin_mainnet_blockchain_info(raw: TransformArgs) -> HttpRequestResult {
    endpoint_bitcoin_mainnet_blockchain_info().transform(raw)
}

#[query]
fn transform_bitcoin_mainnet_blockstream_info(raw: TransformArgs) -> HttpRequestResult {
    endpoint_bitcoin_mainnet_blockstream_info().transform(raw)
}

#[query]
fn transform_bitcoin_mempool(raw: TransformArgs) -> HttpRequestResult {
    endpoint_bitcoin_mainnet_mempool().transform(raw)
}

#[query]
fn transform_dogecoin_canister(raw: TransformArgs) -> HttpRequestResult {
    endpoint_dogecoin_canister().transform(raw)
}

#[query]
fn transform_dogecoin_mainnet_api_blockchair_com(raw: TransformArgs) -> HttpRequestResult {
    endpoint_dogecoin_mainnet_api_blockchair_com().transform(raw)
}

#[query]
fn transform_dogecoin_mainnet_api_blockcypher_com(raw: TransformArgs) -> HttpRequestResult {
    endpoint_dogecoin_mainnet_api_blockcypher_com().transform(raw)
}

#[query]
fn transform_dogecoin_mainnet_psy_protocol(raw: TransformArgs) -> HttpRequestResult {
    endpoint_dogecoin_mainnet_psy_protocol().transform(raw)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::{Canister, Config};
    use crate::types::InitArg;

    #[test]
    fn init_with_bitcoin_testnet_uses_testnet_config() {
        let canister = Canister::BitcoinTestnet;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);
        assert_eq!(get_config(), Config::for_target(canister));
    }

    #[test]
    fn init_with_bitcoin_mainnet_uses_mainnet_config() {
        let canister = Canister::BitcoinMainnet;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);
        assert_eq!(get_config(), Config::for_target(canister));
    }

    #[test]
    fn init_with_bitcoin_mainnet_staging_uses_mainnet_staging_config() {
        let canister = Canister::BitcoinMainnetStaging;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);
        assert_eq!(get_config(), Config::for_target(canister));
    }

    #[test]
    fn init_with_dogecoin_mainnet_uses_mainnet_config() {
        let canister = Canister::DogecoinMainnet;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);
        assert_eq!(get_config(), Config::for_target(canister));
    }

    #[test]
    fn init_with_dogecoin_mainnet_staging_uses_mainnet_staging_config() {
        let canister = Canister::DogecoinMainnetStaging;
        let init_arg = WatchdogArg::Init(InitArg { target: canister });
        init(init_arg);
        assert_eq!(get_config(), Config::for_target(canister));
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
        assert_eq!(config_after, Config::for_target(canister));
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
