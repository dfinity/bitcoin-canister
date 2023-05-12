use ic_btc_interface::{Config, Flag, SetConfigRequest};
use ic_cdk::export::candid::CandidType;
use ic_cdk_macros::{init, post_upgrade, query, update};
use ic_metrics_encoder::MetricsEncoder;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::cell::RefCell;

thread_local! {
    static API_ACCESS: RefCell<Flag> = RefCell::new(Flag::Enabled);
}

type HeaderField = (String, String);

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct CandidHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderField>,
    pub body: ByteBuf,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct CandidHttpResponse {
    pub status_code: u16,
    pub headers: Vec<HeaderField>,
    pub body: ByteBuf,
}

#[init]
fn init() {}

#[post_upgrade]
fn post_upgrade() {
    init()
}

#[query]
fn get_config() -> Config {
    Config {
        api_access: API_ACCESS.with(|cell| *cell.borrow()),
        ..Default::default()
    }
}

#[update]
async fn set_config(request: SetConfigRequest) {
    if let Some(flag) = request.api_access {
        API_ACCESS.with(|cell| *cell.borrow_mut() = flag);
    }
}

/// Processes external HTTP requests.
#[query]
pub fn http_request(request: CandidHttpRequest) -> CandidHttpResponse {
    let parts: Vec<&str> = request.url.split('?').collect();
    match parts[0] {
        "/metrics" => get_metrics(),
        _ => CandidHttpResponse {
            status_code: 404,
            headers: vec![],
            body: ByteBuf::from(String::from("Not found.")),
        },
    }
}

pub fn get_metrics() -> CandidHttpResponse {
    let now = ic_cdk::api::time();
    let mut writer = MetricsEncoder::new(vec![], (now / 1_000_000) as i64);
    match encode_metrics(&mut writer) {
        Ok(()) => {
            let body = writer.into_inner();
            CandidHttpResponse {
                status_code: 200,
                headers: vec![
                    (
                        "Content-Type".to_string(),
                        "text/plain; version=0.0.4".to_string(),
                    ),
                    ("Content-Length".to_string(), body.len().to_string()),
                ],
                body: ByteBuf::from(body),
            }
        }
        Err(err) => CandidHttpResponse {
            status_code: 500,
            headers: vec![],
            body: ByteBuf::from(format!("Failed to encode metrics: {}", err)),
        },
    }
}

fn encode_metrics(w: &mut MetricsEncoder<Vec<u8>>) -> std::io::Result<()> {
    w.encode_gauge("main_chain_height", 700_123.0, "Height of the main chain.")?;

    Ok(())
}

fn main() {}
