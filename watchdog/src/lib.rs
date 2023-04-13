mod bitcoin_block_apis;
mod endpoints;
mod http;

#[cfg(test)]
mod test_utils;

use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::endpoints::*;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};

// TODO: this is a temporary debug method, cleanup before rolling out to prod.
// This method allows to check the data returned by all the APIs.
#[ic_cdk_macros::update]
pub async fn fetch_data() -> String {
    let api_providers = [
        //BitcoinBlockApi::ApiBitapsCom,
        BitcoinBlockApi::ApiBlockchairCom,
        BitcoinBlockApi::ApiBlockcypherCom,
        BitcoinBlockApi::BitcoinCanister,
        BitcoinBlockApi::BlockchainInfo,
        BitcoinBlockApi::BlockstreamInfo,
        //BitcoinBlockApi::ChainApiBtcCom,
    ];
    let futures = api_providers
        .iter()
        .map(|api| api.fetch_data())
        .collect::<Vec<_>>();
    let results = futures::future::join_all(futures).await;
    let mut result = String::new();
    for (api, value) in api_providers.iter().zip(results.iter()) {
        result.push_str(format!("{:?} => ", api).as_str());
        result.push_str(&value.to_string());
        result.push('\n');
    }
    result
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

#[cfg(target_arch = "wasm32")]
pub fn print(msg: &str) {
    ic_cdk::api::print(msg);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn print(msg: &str) {
    println!("{}", msg);
}
