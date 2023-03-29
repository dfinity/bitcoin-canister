use ic_btc_canister::types::{Config, HttpRequest, HttpResponse, SetConfigRequest};
use ic_btc_interface::{
    GetBalanceRequest, GetCurrentFeePercentilesRequest, GetUtxosRequest, MillisatoshiPerByte,
    SendTransactionRequest,
};
use ic_cdk::api::call::{reject, reply};
use ic_cdk_macros::{heartbeat, init, post_upgrade, pre_upgrade, query, update};
#[init]
fn init(config: Config) {
    ic_btc_canister::init(config);
}

#[pre_upgrade]
fn pre_upgrade() {
    ic_btc_canister::pre_upgrade();
}

#[post_upgrade]
fn post_upgrade() {
    ic_btc_canister::post_upgrade();
}

#[heartbeat]
async fn heartbeat() {
    ic_btc_canister::heartbeat().await
}

#[update(manual_reply = true)]
pub fn bitcoin_get_balance(request: GetBalanceRequest) {
    match ic_btc_canister::get_balance(request) {
        Ok(response) => reply((response,)),
        Err(e) => reject(format!("get_balance failed: {:?}", e).as_str()),
    }
}

#[update(manual_reply = true)]
pub fn bitcoin_get_utxos(request: GetUtxosRequest) {
    match ic_btc_canister::get_utxos(request) {
        Ok(response) => reply((response,)),
        Err(e) => reject(format!("get_utxos failed: {:?}", e).as_str()),
    };
}

#[update(manual_reply = true)]
async fn bitcoin_send_transaction(request: SendTransactionRequest) {
    match ic_btc_canister::send_transaction(request).await {
        Ok(_) => reply(()),
        Err(e) => reject(format!("send_transaction failed: {:?}", e).as_str()),
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

fn main() {}
