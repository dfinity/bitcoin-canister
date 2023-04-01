use crate::remote_api::http::{create_request, fetch_body};
use crate::remote_api::storage;
use crate::types::BlockHeight;
use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;

pub struct BlockchainInfo {}

impl BlockchainInfo {
    /// The host name of the remote API.
    pub fn host() -> &'static str {
        "blockchain.info"
    }

    /// The URL of the remote API.
    pub fn url() -> String {
        let host = Self::host();
        format!("https://{host}/q/getblockcount")
    }

    /// Reads the block height from the local storage.
    pub fn get_height() -> Option<BlockHeight> {
        storage::get(Self::host())
    }

    /// Stores the block height in the local storage.
    fn set_height(height: BlockHeight) {
        storage::insert(Self::host(), height)
    }

    /// Creates the HTTP request.
    fn create_request() -> CanisterHttpRequestArgument {
        create_request(Self::host(), Self::url(), None, None)
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
    use crate::ic_http_mock::{create_response, mock};

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
        let mocked_response = create_response().status(200).body(RESPONSE).build();
        mock(&request, &mocked_response);

        BlockchainInfo::fetch().await;

        assert_eq!(BlockchainInfo::get_height(), Some(BlockHeight::new(700001)));
    }
}
