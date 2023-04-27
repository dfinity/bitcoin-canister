use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::config::BitcoinNetwork;
use crate::health::HeightStatus;
use crate::types::CandidHttpResponse;
use ic_metrics_encoder::MetricsEncoder;
use serde_bytes::ByteBuf;
use std::collections::HashMap;

/// Returns the metrics in the Prometheus format.
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

/// Encodes the metrics in the Prometheus format.
fn encode_metrics(w: &mut MetricsEncoder<Vec<u8>>) -> std::io::Result<()> {
    const NO_HEIGHT: f64 = -1.0;
    const NO_HEIGHT_DIFF: f64 = -1_000.0;

    let (mainnet, testnet) = match crate::storage::get_config().bitcoin_network {
        BitcoinNetwork::Mainnet => (1.0, 0.0),
        BitcoinNetwork::Testnet => (0.0, 1.0),
    };
    w.gauge_vec("bitcoin_network", "Bitcoin network.")?
        .value(&[("network", "mainnet")], mainnet)?
        .value(&[("network", "testnet")], testnet)?;

    let health = crate::health::health_status();
    w.encode_gauge(
        "bitcoin_canister_height",
        health.height_source.map(|x| x as f64).unwrap_or(NO_HEIGHT),
        "Main chain height of the Bitcoin canister.",
    )?;
    w.encode_gauge(
        "height_target",
        health.height_target.map(|x| x as f64).unwrap_or(NO_HEIGHT),
        "Height target derived from explorer heights.",
    )?;
    w.encode_gauge(
        "height_diff",
        health
            .height_diff
            .map(|x| x as f64)
            .unwrap_or(NO_HEIGHT_DIFF),
        "Difference between Bitcoin canister height and target height.",
    )?;

    let (not_enough_data, ok, ahead, behind) = match health.height_status {
        HeightStatus::NotEnoughData => (1.0, 0.0, 0.0, 0.0),
        HeightStatus::Ok => (0.0, 1.0, 0.0, 0.0),
        HeightStatus::Ahead => (0.0, 0.0, 1.0, 0.0),
        HeightStatus::Behind => (0.0, 0.0, 0.0, 1.0),
    };
    w.gauge_vec("height_status", "Bitcoin canister height status.")?
        .value(&[("code", "not_enough_data")], not_enough_data)?
        .value(&[("code", "ok")], ok)?
        .value(&[("code", "ahead")], ahead)?
        .value(&[("code", "behind")], behind)?;

    let mut available_explorers = HashMap::new();
    for explorer in health.explorers {
        available_explorers.insert(explorer.provider.clone(), explorer);
    }
    let mut gauge = w.gauge_vec("explorer_height", "Heights from the explorers.")?;
    let bitcoin_network = crate::storage::get_config().bitcoin_network;
    for explorer in BitcoinBlockApi::network_explorers(bitcoin_network) {
        let height = match available_explorers.get(&explorer) {
            None => NO_HEIGHT,
            Some(explorer) => explorer.height.map(|x| x as f64).unwrap_or(NO_HEIGHT),
        };
        gauge = gauge.value(&[("explorer", &explorer.to_string())], height)?;
    }

    Ok(())
}
