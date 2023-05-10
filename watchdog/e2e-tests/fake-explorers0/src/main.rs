use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde_json::json;
use std::convert::Infallible;

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), response_text(req.uri().path())) {
        (&Method::GET, Some(text)) => Ok(Response::new(Body::from(text))),
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main]
async fn main() {
    let make_service =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let addr = ([127, 0, 0, 1], 8080).into();

    let server = Server::bind(&addr).serve(make_service);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

fn response_text(path: &str) -> Option<String> {
    let height = 700_100;
    let response = match path {
        "/status" => String::from("OK"),
        "/api.bitaps.com/btc/v1/blockchain/block/last"
        | "/api.bitaps.com/btc/testnet/v1/blockchain/block/last" => api_bitaps_com_response(height),
        "/api.blockchair.com/bitcoin/stats" | "/api.blockchair.com/bitcoin/testnet/stats" => {
            api_blockchair_com_response(height)
        }
        "/api.blockcypher.com/v1/btc/main" | "/api.blockcypher.com/v1/btc/test3" => {
            api_blockcypher_com_response(height)
        }
        "/ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics"
        | "/g4xu7-jiaaa-aaaan-aaaaq-cai.raw.ic0.app/metrics" => bitcoin_canister_response(height),
        "/blockchain.info/q/latesthash" => blockchain_info_hash_response(),
        "/blockchain.info/q/getblockcount" => blockchain_info_height_response(height),
        "/blockstream.info/api/blocks/tip/hash"
        | "/blockstream.info/testnet/api/blocks/tip/hash" => blockstream_info_hash_response(),
        "/blockstream.info/api/blocks/tip/height"
        | "/blockstream.info/testnet/api/blocks/tip/height" => {
            blockstream_info_height_response(height)
        }
        "/chain.api.btc.com/v3/block/latest" => chain_api_btc_com_response(height),
        _ => return None,
    };

    Some(response)
}

fn api_bitaps_com_response(height: u64) -> String {
    serde_json::to_string_pretty(&json!({
        "data": {
            "height": height
        },
    }))
    .unwrap()
}

fn api_blockchair_com_response(height: u64) -> String {
    serde_json::to_string_pretty(&json!({
        "data": {
            "best_block_height": height
        },
    }))
    .unwrap()
}

fn api_blockcypher_com_response(height: u64) -> String {
    serde_json::to_string_pretty(&json!({ "height": height })).unwrap()
}

fn bitcoin_canister_response(height: u64) -> String {
    format!(r#"main_chain_height {height} 1680014894644"#)
}

fn blockchain_info_hash_response() -> String {
    r#"0000000000000000000aaa444444444444444444444444444444444444444444"#.to_string()
}

fn blockchain_info_height_response(height: u64) -> String {
    format!(r#"{height}"#)
}

fn blockstream_info_hash_response() -> String {
    r#"0000000000000000000aaa444444444444444444444444444444444444444444"#.to_string()
}

fn blockstream_info_height_response(height: u64) -> String {
    format!(r#"{height}"#)
}

fn chain_api_btc_com_response(height: u64) -> String {
    serde_json::to_string_pretty(&json!({
        "data": {
            "height": height
        },
    }))
    .unwrap()
}
