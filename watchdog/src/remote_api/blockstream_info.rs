use crate::remote_api::http::{create_request, fetch_body};
use crate::remote_api::storage;
use crate::types::BlockHeight;
use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;

pub struct BlockstreamInfo {}

impl BlockstreamInfo {
    /// The host name of the remote API.
    pub fn host() -> String {
        "blockstream.info".to_string()
    }

    /// The URL of the remote API.
    pub fn url() -> String {
        let host = Self::host();
        format!("https://{host}/api/blocks/tip/height")
    }

    /// Reads the block height from the local storage.
    pub fn get_height() -> Option<BlockHeight> {
        storage::get(&Self::host())
    }

    /// Stores the block height in the local storage.
    fn set_height(height: BlockHeight) {
        storage::insert(&Self::host(), height)
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

    #[test]
    fn test_request_url() {
        assert_eq!(
            BlockstreamInfo::create_request().url,
            "https://blockstream.info/api/blocks/tip/height"
        );
    }

    #[test]
    fn test_has_no_transform() {
        assert!(BlockstreamInfo::create_request().transform.is_none());
    }
}
