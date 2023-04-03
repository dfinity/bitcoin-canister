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
fn transform_api_blockchair_com(raw: TransformArgs) -> HttpResponse {
    apply_to_body_json(raw, ApiBlockchairCom::transform)
}

pub struct ApiBlockchairCom {}

impl ApiBlockchairCom {
    /// The host name of the remote API.
    pub fn host() -> &'static str {
        "api.blockchair.com"
    }

    /// The URL of the remote API.
    pub fn url() -> String {
        let host = Self::host();
        format!("https://{host}/bitcoin/stats")
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
            .get("best_block_height")
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
            Some(build_transform_context(
                transform_api_blockchair_com,
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

    // https://api.blockchair.com/bitcoin/stats
    const RESPONSE: &str = r#"{
        "data":
        {
            "blocks":783771,
            "transactions":820266066,
            "outputs":2309684029,
            "circulation":1933603979497096,
            "blocks_24h":148,
            "transactions_24h":370690,
            "difficulty":46843400286277,
            "volume_24h":97687710547510,
            "mempool_transactions":29979,
            "mempool_size":203718813,
            "mempool_tps":4.433333333333334,
            "mempool_total_fee_usd":52388.2163,
            "best_block_height":783770,
            "best_block_hash":"00000000000000000001d03f94ae0c307a708c48253d7b24ce1d675e65b7fe08",
            "best_block_time":"2023-04-03 14:04:50",
            "blockchain_size":470319339145,
            "average_transaction_fee_24h":6780,
            "inflation_24h":92500000000,
            "median_transaction_fee_24h":3495,
            "cdd_24h":5327187.228927112,
            "mempool_outputs":637712,
            "largest_transaction_24h":
            {
                "hash":"0fde94d2ca0eb734f83c166626bf22dea861deb6aba69e7d1c28f1171a922f13",
                "value_usd":427008416
            },
            "nodes":7718,
            "hashrate_24h":"345095835785586196564",
            "inflation_usd_24h":26150675,
            "average_transaction_fee_usd_24h":1.9170246384876852,
            "median_transaction_fee_usd_24h":0.9880714500000001,
            "market_price_usd":28271,
            "market_price_btc":1,
            "market_price_usd_change_24h_percentage":-0.15658,
            "market_cap_usd":546793120160,
            "market_dominance_percentage":44.66,
            "next_retarget_time_estimate":"2023-04-06 16:32:29",
            "next_difficulty_estimate":44336619371627,
            "countdowns":[],
            "suggested_transaction_fee_per_byte_sat":21,
            "hodling_addresses":45818990
        }    
    }"#;

    #[test]
    fn test_request_url() {
        assert_eq!(
            ApiBlockchairCom::create_request().url,
            "https://api.blockchair.com/bitcoin/stats"
        );
    }

    #[test]
    fn test_has_transform() {
        assert!(ApiBlockchairCom::create_request().transform.is_some());
    }

    #[tokio::test]
    async fn test_fetch() {
        let request = ApiBlockchairCom::create_request();
        let mocked_response = create_response().status(200).body(RESPONSE).build();
        mock(&request, &mocked_response);

        ApiBlockchairCom::fetch().await;

        assert_eq!(
            ApiBlockchairCom::get_height(),
            Some(BlockHeight::new(783770))
        );
    }
}
