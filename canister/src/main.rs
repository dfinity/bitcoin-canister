use ic_btc_canister::types::{Config, HttpRequest, HttpResponse, SetConfigRequest};
use ic_btc_types::{
    GetBalanceRequest, GetCurrentFeePercentilesRequest, GetUtxosRequest, GetUtxosResponse,
    MillisatoshiPerByte, Satoshi,
};
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

#[update]
pub fn bitcoin_get_balance(request: GetBalanceRequest) -> Satoshi {
    ic_btc_canister::get_balance(request)
}

#[update]
pub fn bitcoin_get_utxos(request: GetUtxosRequest) -> GetUtxosResponse {
    ic_btc_canister::get_utxos(request)
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
pub fn set_config(request: SetConfigRequest) {
    ic_btc_canister::set_config(request)
}

#[query]
pub fn http_request(request: HttpRequest) -> HttpResponse {
    ic_btc_canister::http_request(request)
}

fn main() {}
