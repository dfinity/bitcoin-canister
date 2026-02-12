use ic_btc_canister::types::{HttpRequest, HttpResponse};
use ic_btc_canister::CanisterArg;
use ic_btc_interface::{
    Config, GetBalanceRequest, GetBlockHeadersRequest, GetBlockHeadersResponse,
    GetCurrentFeePercentilesRequest, GetUtxosRequest, GetUtxosResponse, MillisatoshiPerByte,
    Satoshi, SendTransactionRequest, SetConfigRequest,
};
use ic_cdk::{
    api::{msg_reject, msg_reply},
    heartbeat, init, inspect_message, post_upgrade, pre_upgrade, query, update,
};
use std::marker::PhantomData;

#[cfg(target_arch = "wasm32")]
mod printer;

fn hook() {
    #[cfg(target_arch = "wasm32")]
    printer::hook();
}

#[init]
fn init(canister_arg: CanisterArg) {
    hook();
    match canister_arg {
        CanisterArg::Init(init_config) => {
            ic_btc_canister::init(init_config);
        }
        CanisterArg::Upgrade(_) => {
            panic!("expected Init arguments got Upgrade arguments");
        }
    }
}

#[pre_upgrade]
fn pre_upgrade() {
    ic_btc_canister::pre_upgrade();
}

#[post_upgrade]
fn post_upgrade(canister_arg: Option<CanisterArg>) {
    hook();
    let mut config_update: Option<SetConfigRequest> = None;
    if let Some(canister_arg) = canister_arg {
        config_update = match canister_arg {
            CanisterArg::Init(_) => {
                panic!("expected Upgrade arguments got Init arguments");
            }
            CanisterArg::Upgrade(args) => args,
        }
    }
    ic_btc_canister::post_upgrade(config_update);
}

#[heartbeat]
async fn heartbeat() {
    ic_btc_canister::heartbeat().await
}

#[update(manual_reply = true)]
pub fn bitcoin_get_balance(request: GetBalanceRequest) -> PhantomData<Satoshi> {
    match ic_btc_canister::get_balance(request) {
        Ok(response) => msg_reply(candid::encode_one(response).unwrap()),
        Err(e) => msg_reject(format!("get_balance failed: {:?}", e).as_str()),
    };
    PhantomData
}

#[query(manual_reply = true)]
pub fn bitcoin_get_balance_query(request: GetBalanceRequest) -> PhantomData<Satoshi> {
    if ic_cdk::api::data_certificate().is_none() {
        msg_reject("get_balance_query cannot be called in replicated mode");
    } else {
        match ic_btc_canister::get_balance_query(request) {
            Ok(response) => msg_reply(candid::encode_one(response).unwrap()),
            Err(e) => msg_reject(format!("get_balance_query failed: {:?}", e).as_str()),
        }
    }
    PhantomData
}

#[update(manual_reply = true)]
pub fn bitcoin_get_utxos(request: GetUtxosRequest) -> PhantomData<GetUtxosResponse> {
    match ic_btc_canister::get_utxos(request) {
        Ok(response) => msg_reply(candid::encode_one(response).unwrap()),
        Err(e) => msg_reject(format!("get_utxos failed: {:?}", e).as_str()),
    }
    PhantomData
}

#[query(manual_reply = true)]
pub fn bitcoin_get_utxos_query(request: GetUtxosRequest) -> PhantomData<GetUtxosResponse> {
    if ic_cdk::api::data_certificate().is_none() {
        msg_reject("get_utxos_query cannot be called in replicated mode");
    } else {
        match ic_btc_canister::get_utxos_query(request) {
            Ok(response) => msg_reply(candid::encode_one(response).unwrap()),
            Err(e) => msg_reject(format!("get_utxos_query failed: {:?}", e).as_str()),
        }
    }
    PhantomData
}

#[update(manual_reply = true)]
pub fn bitcoin_get_block_headers(
    request: GetBlockHeadersRequest,
) -> PhantomData<GetBlockHeadersResponse> {
    match ic_btc_canister::get_block_headers(request) {
        Ok(response) => msg_reply(candid::encode_one(response).unwrap()),
        Err(e) => msg_reject(format!("get_block_headers failed: {:?}", e).as_str()),
    }
    PhantomData
}

#[update(manual_reply = true)]
async fn bitcoin_send_transaction(request: SendTransactionRequest) -> PhantomData<()> {
    match ic_btc_canister::send_transaction(request).await {
        Ok(_) => msg_reply(candid::encode_one(()).unwrap()),
        Err(e) => msg_reject(format!("send_transaction failed: {:?}", e).as_str()),
    }
    PhantomData
}

#[update]
pub fn bitcoin_get_current_fee_percentiles(
    request: GetCurrentFeePercentilesRequest,
) -> Vec<MillisatoshiPerByte> {
    ic_btc_canister::get_current_fee_percentiles(request)
}

#[query]
pub fn get_config() -> Config {
    ic_btc_canister::get_config()
}

#[update]
fn set_config(request: SetConfigRequest) {
    ic_btc_canister::set_config(request)
}

#[query]
pub fn get_blockchain_info() -> ic_btc_canister::types::BlockchainInfo {
    ic_btc_canister::get_blockchain_info()
}

#[query]
pub fn http_request(request: HttpRequest) -> HttpResponse {
    ic_btc_canister::http_request(request)
}

#[inspect_message]
fn inspect_message() {
    // Reject calls to the query endpoints as they are not supported in replicated mode.
    let inspected_method_name = ic_cdk::api::msg_method_name();
    if inspected_method_name.as_str() != "bitcoin_get_balance_query"
        && inspected_method_name.as_str() != "bitcoin_get_utxos_query"
    {
        ic_cdk::api::accept_message();
    }
}

// Expose a method to know if canbench is included in the binary or not.
// This is used in a test to ensure that canbench is _not_ included in the
// production binary.
#[cfg(feature = "canbench-rs")]
#[update]
pub fn has_canbench() -> bool {
    true
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{init, post_upgrade};
    use ic_btc_canister::CanisterArg;
    use ic_btc_interface::{InitConfig, SetConfigRequest};

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

    #[test]
    #[should_panic]
    fn init_panics_with_upgrade_args() {
        init(CanisterArg::Upgrade(Some(SetConfigRequest::default())));
    }

    #[test]
    #[should_panic]
    fn upgrade_panics_with_init_args() {
        post_upgrade(Some(CanisterArg::Init(InitConfig::default())));
    }
}
