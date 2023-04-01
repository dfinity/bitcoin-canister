use crate::remote_api::http::{
    apply_to_body_json, build_transform_context, create_request, fetch_body,
};
use crate::remote_api::storage;
use crate::types::BlockHeight;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpResponse, TransformArgs,
};
use serde_json::json;

#[ic_cdk_macros::query]
fn transform_api_bitaps_com(raw: TransformArgs) -> HttpResponse {
    apply_to_body_json(raw, ApiBitapsCom::transform)
}

pub struct ApiBitapsCom {}

impl ApiBitapsCom {
    pub fn host() -> &'static str {
        "api.bitaps.com"
    }

    pub fn url() -> String {
        let host = Self::host();
        format!("https://{host}/btc/v1/blockchain/block/last")
    }

    pub fn get_height() -> Option<BlockHeight> {
        storage::get(Self::host())
    }

    fn set_height(height: BlockHeight) {
        storage::insert(Self::host(), height)
    }

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

    fn create_request() -> CanisterHttpRequestArgument {
        create_request(
            Self::host(),
            Self::url(),
            None,
            Some(build_transform_context(transform_api_bitaps_com, vec![])),
        )
    }

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
    use crate::ic_http_mock::{mock, create_response};

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
        let mocked_response = create_response()
            .status(200)
            .body(RESPONSE)
            .build();
        mock(&request, &mocked_response);

        ApiBitapsCom::fetch().await;

        assert_eq!(ApiBitapsCom::get_height(), Some(BlockHeight::new(700004)));
    }
}
