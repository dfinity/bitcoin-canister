use crate::remote_api::http::{
    apply_to_body_json, build_transform_context, create_request, fetch_body,
};
use crate::remote_api::storage;
use crate::types::BlockHeight;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpResponse, TransformArgs,
};
use serde_json::json;

pub struct ChainApiBtcCom {}

/// The transform function for the remote API.
#[ic_cdk_macros::query]
fn transform_chain_api_btc_com(raw: TransformArgs) -> HttpResponse {
    apply_to_body_json(raw, ChainApiBtcCom::transform)
}

impl ChainApiBtcCom {
    /// The host name of the remote API.
    pub fn host() -> &'static str {
        "chain.api.btc.com"
    }

    /// The URL of the remote API.
    pub fn url() -> String {
        let host = Self::host();
        format!("https://{host}/v3/block/latest")
    }

    /// Reads the block height from the local storage.
    pub fn get_height() -> Option<BlockHeight> {
        storage::get(Self::host())
    }

    /// Stores the block height in the local storage.
    fn set_height(height: BlockHeight) {
        storage::insert(Self::host(), height)
    }

    /// The transform function for the JSON body.
    fn transform(json: serde_json::Value) -> serde_json::Value {
        let empty = json!({});
        match json
            .get("data")
            .unwrap_or(&empty)
            .get("height")
            .and_then(BlockHeight::from_json)
        {
            Some(x) => x.as_json(),
            None => empty,
        }
    }

    /// Creates the HTTP request.
    fn create_request() -> CanisterHttpRequestArgument {
        create_request(
            Self::host(),
            Self::url(),
            None,
            Some(build_transform_context(transform_chain_api_btc_com, vec![])),
        )
    }

    /// Fetches the block height from the remote API and stores it in the local storage.
    pub async fn fetch() {
        let request = Self::create_request();
        let body = fetch_body(request).await;

        match body {
            Err(_) => (),
            Ok(body) => match BlockHeight::from_string(body) {
                None => (),
                Some(height) => {
                    Self::set_height(height);
                }
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_request_url() {
        assert_eq!(
            ChainApiBtcCom::create_request().url,
            "https://chain.api.btc.com/v3/block/latest"
        );
    }

    #[test]
    fn test_has_transform() {
        assert!(ChainApiBtcCom::create_request().transform.is_some());
    }
}
