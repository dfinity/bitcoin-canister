use crate::remote_api::http::{
    apply_to_body_json, build_transform_context, create_request, fetch_body,
};
use crate::remote_api::storage;
use crate::types::BlockHeight;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpResponse, TransformArgs,
};
use serde_json::json;

/// The transform function for the remote API.
#[ic_cdk_macros::query]
fn transform_api_bitaps_com(raw: TransformArgs) -> HttpResponse {
    apply_to_body_json(raw, ApiBitapsCom::transform)
}

pub struct ApiBitapsCom {}

impl ApiBitapsCom {
    /// The host name of the remote API.
    pub fn host() -> &'static str {
        "api.bitaps.com"
    }

    /// The URL of the remote API.
    pub fn url() -> String {
        let host = Self::host();
        format!("https://{host}/btc/v1/blockchain/block/last")
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
            Some(build_transform_context(transform_api_bitaps_com, vec![])),
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
    use ic_http_mock::{create_response, mock};

    // https://api.bitaps.com/btc/v1/blockchain/block/last
    const RESPONSE: &str = r#"{
        "data": {
            "height": 700004,
            "hash": "0000000000000000000aaa444444444444444444444444444444444444444444",
            "header": "AGAAILqkI+SFlsu4FRCwVNiwU3Eku+N/g9sEAAAAAAAAAAAAH1tWFGtObfxfaOeXVwH9txRFHWS4V+N24n9AyliR1S4Yvghko4kGFwdzNef9XA4=",
            "adjustedTimestamp": 1678294552
        },
        "time": 0.0018
    }"#;

    #[test]
    fn test_request_url() {
        assert_eq!(
            ApiBitapsCom::create_request().url,
            "https://api.bitaps.com/btc/v1/blockchain/block/last"
        );
    }

    #[test]
    fn test_has_transform() {
        assert!(ApiBitapsCom::create_request().transform.is_some());
    }

    #[tokio::test]
    async fn test_fetch() {
        let request = ApiBitapsCom::create_request();
        let mocked_response = create_response().status(200).body(RESPONSE).build();
        mock(&request, &mocked_response);

        ApiBitapsCom::fetch().await;

        assert_eq!(ApiBitapsCom::get_height(), Some(BlockHeight::new(700004)));
    }
}