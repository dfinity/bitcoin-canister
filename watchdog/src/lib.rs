mod bitcoin_block_apis;
mod endpoints;
mod http;

#[cfg(test)]
mod test_utils;

use crate::endpoints::Endpoint::*;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};

// TODO: cleanup.
#[ic_cdk_macros::query]
pub fn version() -> String {
    String::from("v.0.1.0")
}

#[ic_cdk_macros::query]
fn transform_api_bitaps_com_block(raw: TransformArgs) -> HttpResponse {
    ApiBitapsComBlock.get().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_api_blockchair_com_block(raw: TransformArgs) -> HttpResponse {
    ApiBlockchairComBlock.get().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_api_blockcypher_com_block(raw: TransformArgs) -> HttpResponse {
    ApiBlockcypherComBlock.get().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_bitcoin_canister(raw: TransformArgs) -> HttpResponse {
    BitcoinCanister.get().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_blockchain_info_hash(raw: TransformArgs) -> HttpResponse {
    BlockchainInfoHash.get().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_blockchain_info_height(raw: TransformArgs) -> HttpResponse {
    BlockchainInfoHeight.get().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_blockstream_info_hash(raw: TransformArgs) -> HttpResponse {
    BlockstreamInfoHash.get().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_blockstream_info_height(raw: TransformArgs) -> HttpResponse {
    BlockstreamInfoHeight.get().transform(raw)
}

#[ic_cdk_macros::query]
fn transform_chain_api_btc_com_block(raw: TransformArgs) -> HttpResponse {
    ChainApiBtcComBlock.get().transform(raw)
}

#[cfg(target_arch = "wasm32")]
pub fn print(msg: &str) {
    ic_cdk::api::print(msg);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn print(msg: &str) {
    println!("{}", msg);
}
