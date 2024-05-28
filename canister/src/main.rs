use ic_btc_canister::types::{HttpRequest, HttpResponse};
use ic_btc_interface::{
    Config, GetBalanceRequest, GetBlockHeadersRequest, GetBlockHeadersResponse,
    GetCurrentFeePercentilesRequest, GetUtxosRequest, GetUtxosResponse, MillisatoshiPerByte,
    Satoshi, SendTransactionRequest, SetConfigRequest,
};
use ic_cdk::api::call::ManualReply;
use ic_cdk_macros::{heartbeat, init, inspect_message, post_upgrade, pre_upgrade, query, update};

#[cfg(target_arch = "wasm32")]
mod printer;

fn hook() {
    #[cfg(target_arch = "wasm32")]
    printer::hook();
}

#[init]
fn init(config: Config) {
    hook();
    ic_btc_canister::init(config);
}

#[pre_upgrade]
fn pre_upgrade() {
    ic_btc_canister::pre_upgrade();
}

#[post_upgrade]
fn post_upgrade() {
    hook();
    ic_btc_canister::post_upgrade();
}

#[heartbeat]
async fn heartbeat() {
    ic_btc_canister::heartbeat().await
}

#[update(manual_reply = true)]
pub fn bitcoin_get_balance(request: GetBalanceRequest) -> ManualReply<Satoshi> {
    match ic_btc_canister::get_balance(request) {
        Ok(response) => ManualReply::one(response),
        Err(e) => ManualReply::reject(format!("get_balance failed: {:?}", e).as_str()),
    }
}

#[query(manual_reply = true)]
pub fn bitcoin_get_balance_query(request: GetBalanceRequest) -> ManualReply<Satoshi> {
    if ic_cdk::api::data_certificate().is_none() {
        return ManualReply::reject("get_balance_query cannot be called in replicated mode");
    }
    match ic_btc_canister::get_balance_query(request) {
        Ok(response) => ManualReply::one(response),
        Err(e) => ManualReply::reject(format!("get_balance_query failed: {:?}", e).as_str()),
    }
}

#[update(manual_reply = true)]
pub fn bitcoin_get_utxos(request: GetUtxosRequest) -> ManualReply<GetUtxosResponse> {
    match ic_btc_canister::get_utxos(request) {
        Ok(response) => ManualReply::one(response),
        Err(e) => ManualReply::reject(format!("get_utxos failed: {:?}", e).as_str()),
    }
}

#[query(manual_reply = true)]
pub fn bitcoin_get_utxos_query(request: GetUtxosRequest) -> ManualReply<GetUtxosResponse> {
    if ic_cdk::api::data_certificate().is_none() {
        return ManualReply::reject("get_utxos_query cannot be called in replicated mode");
    }
    match ic_btc_canister::get_utxos_query(request) {
        Ok(response) => ManualReply::one(response),
        Err(e) => ManualReply::reject(format!("get_utxos_query failed: {:?}", e).as_str()),
    }
}

#[update(manual_reply = true)]
pub fn bitcoin_get_block_headers(
    request: GetBlockHeadersRequest,
) -> ManualReply<GetBlockHeadersResponse> {
    match ic_btc_canister::get_block_headers(request) {
        Ok(response) => ManualReply::one(response),
        Err(e) => ManualReply::reject(format!("get_block_headers failed: {:?}", e).as_str()),
    }
}

#[update(manual_reply = true)]
async fn bitcoin_send_transaction(request: SendTransactionRequest) -> ManualReply<()> {
    match ic_btc_canister::send_transaction(request).await {
        Ok(_) => ManualReply::all(()),
        Err(e) => ManualReply::reject(format!("send_transaction failed: {:?}", e).as_str()),
    }
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
async fn set_config(request: SetConfigRequest) {
    ic_btc_canister::set_config(request).await
}

#[query]
pub fn http_request(request: HttpRequest) -> HttpResponse {
    ic_btc_canister::http_request(request)
}

#[inspect_message]
fn inspect_message() {
    // Reject calls to the query endpoints as they are not supported in replicated mode.
    let inspected_method_name = ic_cdk::api::call::method_name();
    if inspected_method_name.as_str() != "bitcoin_get_balance_query"
        && inspected_method_name.as_str() != "bitcoin_get_utxos_query"
    {
        ic_cdk::api::call::accept_message();
    }
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candid_interface_compatibility() {
        use candid_parser::utils::{service_compatible, CandidSource};
        use std::path::PathBuf;

        candid::export_service!();
        let new_interface = __export_service();

        let old_interface =
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("candid.did");

        service_compatible(
            CandidSource::Text(&new_interface),
            CandidSource::File(old_interface.as_path()),
        )
        .expect("The Bitcoin canister interface is not compatible with the candid.did file");
    }
}
