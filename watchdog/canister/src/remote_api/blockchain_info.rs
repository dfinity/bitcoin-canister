use crate::remote_api::http::{create_request, fetch_body};
use crate::remote_api::storage;
use crate::types::BlockHeight;
use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;

pub struct BlockchainInfo {}

impl BlockchainInfo {
    pub fn host() -> &'static str {
        "blockchain.info"
    }

    pub fn url() -> String {
        let host = Self::host();
        format!("https://{host}/q/getblockcount")
    }

    pub fn get_height() -> Option<BlockHeight> {
        storage::get(Self::host())
    }

    fn set_height(height: BlockHeight) {
        storage::insert(Self::host(), height)
    }

    fn create_request() -> CanisterHttpRequestArgument {
        create_request(Self::host(), Self::url(), None, None)
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

    // https://blockchain.info/q/getblockcount
    const RESPONSE: &str = "700001";

    #[test]
    fn test_request_url() {
        assert_eq!(
            BlockchainInfo::create_request().url,
            "https://blockchain.info/q/getblockcount"
        );
    }

    #[test]
    fn test_has_no_transform() {
        assert!(BlockchainInfo::create_request().transform.is_none());
    }

    #[tokio::test]
    async fn test_fetch() {
        let request = BlockchainInfo::create_request();
        let mocked_response = create_response()
            .status(200)
            .body(RESPONSE)
            .build();
        mock(&request, &mocked_response);

        BlockchainInfo::fetch().await;

        assert_eq!(BlockchainInfo::get_height(), Some(BlockHeight::new(700001)));
    }
}
