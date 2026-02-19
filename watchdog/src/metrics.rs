use crate::config::Network;
use crate::{health::HeightStatus, types::CandidHttpResponse};
use ic_btc_interface::Flag;
use ic_metrics_encoder::MetricsEncoder;
use serde_bytes::ByteBuf;
use std::collections::HashMap;

const NO_VALUE: f64 = f64::NAN;

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
    let canister = crate::storage::get_canister();
    let config = crate::storage::get_config();
    let (bitcoin_mainnet, bitcoin_testnet, dogecoin_mainnet) = match canister.network() {
        Network::BitcoinMainnet => (1.0, 0.0, 0.0),
        Network::BitcoinTestnet => (0.0, 1.0, 0.0),
        Network::DogecoinMainnet => (0.0, 0.0, 1.0),
    };
    w.gauge_vec("network", "Network.")?
        .value(&[("network", "bitcoin_mainnet")], bitcoin_mainnet)?
        .value(&[("network", "bitcoin_testnet")], bitcoin_testnet)?
        .value(&[("network", "dogecoin_mainnet")], dogecoin_mainnet)?;
    w.encode_gauge(
        "blocks_behind_threshold",
        config.get_blocks_behind_threshold() as f64,
        "Below this threshold, the canister is considered to be behind.",
    )?;
    w.encode_gauge(
        "blocks_ahead_threshold",
        config.get_blocks_ahead_threshold() as f64,
        "Above this threshold, the canister is considered to be ahead.",
    )?;
    w.encode_gauge(
        "min_explorers",
        config.min_explorers as f64,
        "The minimum number of explorers to compare against.",
    )?;

    let health = crate::health::health_status();
    w.encode_gauge(
        "canister_height",
        health.height_source.map(|x| x as f64).unwrap_or(NO_VALUE),
        "Main chain height of the canister.",
    )?;
    w.encode_gauge(
        "height_target",
        health.height_target.map(|x| x as f64).unwrap_or(NO_VALUE),
        "Height target derived from explorer heights.",
    )?;
    w.encode_gauge(
        "height_diff",
        health.height_diff.map(|x| x as f64).unwrap_or(NO_VALUE),
        "Difference between canister height and target height.",
    )?;

    let (not_enough_data, ok, ahead, behind) = match health.height_status {
        HeightStatus::NotEnoughData => (1.0, 0.0, 0.0, 0.0),
        HeightStatus::Ok => (0.0, 1.0, 0.0, 0.0),
        HeightStatus::Ahead => (0.0, 0.0, 1.0, 0.0),
        HeightStatus::Behind => (0.0, 0.0, 0.0, 1.0),
    };
    w.gauge_vec("height_status", "Canister height status.")?
        .value(&[("code", "not_enough_data")], not_enough_data)?
        .value(&[("code", "ok")], ok)?
        .value(&[("code", "ahead")], ahead)?
        .value(&[("code", "behind")], behind)?;

    let (undefined, enabled, disabled) = match crate::storage::get_api_access_target() {
        None => (1.0, 0.0, 0.0),
        Some(Flag::Enabled) => (0.0, 1.0, 0.0),
        Some(Flag::Disabled) => (0.0, 0.0, 1.0),
    };
    w.gauge_vec(
        "api_access_target",
        "Expected value of the canister API access flag.",
    )?
    .value(&[("flag", "undefined")], undefined)?
    .value(&[("flag", "enabled")], enabled)?
    .value(&[("flag", "disabled")], disabled)?;

    let mut available_explorers = HashMap::new();
    for explorer in health.explorers {
        available_explorers.insert(explorer.provider.clone(), explorer);
    }
    let mut gauge = w.gauge_vec("explorer_height", "Heights from the explorers.")?;
    let mut available_explorers_count: u64 = 0;
    for explorer in &config.explorers {
        let height = available_explorers
            .get(explorer)
            .map_or(NO_VALUE, |block_info| {
                block_info.height.map_or(NO_VALUE, |x| x as f64)
            });
        if !height.is_nan() {
            available_explorers_count += 1;
        }
        gauge = gauge.value(&[("explorer", explorer)], height)?;
    }
    w.encode_gauge(
        "available_explorers",
        available_explorers_count as f64,
        "The number of available explorers to compare against.",
    )?;

    Ok(())
}
