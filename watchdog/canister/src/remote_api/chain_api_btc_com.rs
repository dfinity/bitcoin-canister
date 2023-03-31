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

#[ic_cdk_macros::query]
fn transform_chain_api_btc_com(raw: TransformArgs) -> HttpResponse {
    apply_to_body_json(raw, ChainApiBtcCom::transform)
}

impl ChainApiBtcCom {
    pub fn host() -> &'static str {
        "chain.api.btc.com"
    }

    pub fn url() -> String {
        let host = Self::host();
        format!("https://{host}/v3/block/latest")
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
            Some(build_transform_context(transform_chain_api_btc_com, vec![])),
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
    use crate::ic_http_mock::{mock, HttpResponseBuilder};

    // https://chain.api.btc.com/v3/block/latest
    const RESPONSE: &str = r#"{
        "data": {
            "height":700002,
            "version":538968064,
            "mrkl_root":"fd7a75292e02050465de1ff8a98ea7e0dbe22f6107a3ee89c9de40e32166ad23",
            "timestamp":1679733439,
            "bits":386269758,
            "nonce":110254631,
            "hash":"0000000000000000000aaa222222222222222222222222222222222222222222",
            "prev_block_hash":"0000000000000000000ccc222222222222222222222222222222222222222222",
            "next_block_hash":"0000000000000000000000000000000000000000000000000000000000000000",
            "size":1561960,
            "pool_difficulty":56653058926588,
            "difficulty":46843400286276,
            "difficulty_double":46843400286276.55,
            "tx_count":2957,
            "reward_block":625000000,
            "reward_fees":32773177,
            "confirmations":1,
            "is_orphan":false,
            "curr_max_timestamp":1679733439,
            "is_sw_block":true,
            "stripped_size":810332,
            "sigops":14267,
            "weight":3992956,
            "extras": {
                "pool_name":"PEGA Pool",
                "pool_link":"https://www.pega-pool.com"
            }
        },
        "err_code":0,
        "err_no":0,
        "message":"success",
        "status":"success"
    }"#;

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

    #[tokio::test]
    async fn test_fetch() {
        let request = ChainApiBtcCom::create_request();
        let mocked_response = HttpResponseBuilder::new()
            .status(200)
            .body(RESPONSE)
            .build();
        mock(&request, &mocked_response);

        ChainApiBtcCom::fetch().await;

        assert_eq!(ChainApiBtcCom::get_height(), Some(BlockHeight::new(700002)));
    }
}
