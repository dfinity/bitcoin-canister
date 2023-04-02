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
fn transform_api_block_cypher_com(raw: TransformArgs) -> HttpResponse {
    apply_to_body_json(raw, ApiBlockcypherCom::transform)
}
pub struct ApiBlockcypherCom {}

impl ApiBlockcypherCom {
    /// The host name of the remote API.
    pub fn host() -> &'static str {
        "api.blockcypher.com"
    }

    /// The URL of the remote API.
    pub fn url() -> String {
        let host = Self::host();
        format!("https://{host}/v1/btc/main")
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
        match json.get("height").and_then(BlockHeight::from_json) {
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
            Some(build_transform_context(
                transform_api_block_cypher_com,
                vec![],
            )),
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

    // https://api.blockcypher.com/v1/btc/main
    const RESPONSE: &str = r#"{
        "name": "BTC.main",
        "height": 700003,
        "hash": "0000000000000000000aaa333333333333333333333333333333333333333333",
        "time": "2023-03-25T08:38:41.081949161Z",
        "latest_url": "https://api.blockcypher.com/v1/btc/main/blocks/00000000000000000004f7e4f909f1e9ebbe3db9c94e5165cdda946f8a6a4e72",
        "previous_hash": "0000000000000000000eee333333333333333333333333333333333333333333",
        "previous_url": "https://api.blockcypher.com/v1/btc/main/blocks/00000000000000000001a4e2dc423c9d167fa6ffd9f34bf0c6d919521ef82003",
        "peer_count": 243,
        "unconfirmed_count": 7543,
        "high_fee_per_kb": 33350,
        "medium_fee_per_kb": 19047,
        "low_fee_per_kb": 12258,
        "last_fork_height": 781277,
        "last_fork_hash": "0000000000000000000388f42000fa901c01f2bfae36042bbae133ee430e6485"
    }"#;

    #[test]
    fn test_request_url() {
        assert_eq!(
            ApiBlockcypherCom::create_request().url,
            "https://api.blockcypher.com/v1/btc/main"
        );
    }

    #[test]
    fn test_has_transform() {
        assert!(ApiBlockcypherCom::create_request().transform.is_some());
    }

    #[tokio::test]
    async fn test_fetch() {
        let request = ApiBlockcypherCom::create_request();
        let mocked_response = create_response().status(200).body(RESPONSE).build();
        mock(&request, &mocked_response);

        ApiBlockcypherCom::fetch().await;

        assert_eq!(
            ApiBlockcypherCom::get_height(),
            Some(BlockHeight::new(700003))
        );
    }
}
