use crate::config::Config;
use crate::remote_api::http::{build_transform_context, create_request, fetch_body};
use crate::remote_api::storage;
use crate::types::BlockHeight;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpResponse, TransformArgs,
};
use regex::Regex;
use std::cell::RefCell;

const RE_PATTERN: &str = r"\n\s*main_chain_height (\d+) \d+\n";

// This is a thread-local storage for calculating the regex only once.
thread_local! {
    static REGEX: RefCell<Result<Regex, regex::Error>> = RefCell::new(Regex::new(RE_PATTERN));
}

/// Apply regex rule to a given text.
fn apply(re: &Regex, text: &str) -> Result<String, String> {
    match re.captures(text) {
        None => Err("Regex: no match found.".to_string()),
        Some(cap) => match cap.len() {
            2 => Ok(String::from(&cap[1])),
            x => Err(format!("Regex: expected 1 group exactly, provided {}.", x)),
        },
    }
}

/// The transform function for the remote API.
#[ic_cdk_macros::query]
fn transform_bitcoin_canister(raw: TransformArgs) -> HttpResponse {
    let mut response = HttpResponse {
        status: raw.response.status.clone(),
        ..Default::default()
    };
    if response.status == 200 {
        let body =
            String::from_utf8(raw.response.body).expect("Raw response is not UTF-8 encoded.");
        response.body = BitcoinCanister::transform(body).as_bytes().to_vec();
    } else {
        crate::print(&format!("Received an error: err = {:?}", raw));
    }
    response
}

pub struct BitcoinCanister {}

impl BitcoinCanister {
    /// The host of the remote API.
    pub fn host() -> String {
        Config::default().bitcoin_canister_host
    }

    /// The URL of the remote API.
    pub fn url() -> String {
        let host = Self::host();
        format!("https://{host}/metrics")
    }

    /// Reads the block height from the local storage.
    pub fn get_height() -> Option<BlockHeight> {
        storage::get(&Self::host())
    }

    /// Stores the block height in the local storage.
    fn set_height(height: BlockHeight) {
        storage::insert(&Self::host(), height)
    }

    /// The transform function for the text body.
    fn transform(text: String) -> String {
        match REGEX.with(|x| x.borrow().clone()) {
            Err(_) => String::new(),
            Ok(re) => match apply(&re, &text) {
                Err(_) => String::new(),
                Ok(height) => height,
            },
        }
    }

    /// Creates a request to the remote API.
    fn create_request() -> CanisterHttpRequestArgument {
        create_request(
            Self::host(),
            Self::url(),
            None,
            Some(build_transform_context(transform_bitcoin_canister, vec![])),
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
            BitcoinCanister::create_request().url,
            "https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics"
        );
    }

    #[test]
    fn test_has_transform() {
        assert!(BitcoinCanister::create_request().transform.is_some());
    }
}
