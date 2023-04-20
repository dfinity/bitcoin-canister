use crate::health::StatusCode;
use crate::types::CandidHttpResponse;
use ic_metrics_encoder::MetricsEncoder;
use serde_bytes::ByteBuf;

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

    let health = crate::health::health_status();
    w.encode_gauge(
        "bitcoin_canister_height",
        health.source_height.map(|x| x as f64).unwrap_or(NO_HEIGHT),
        "Height of the main chain of the Bitcoin canister.",
    )?;
    w.encode_gauge(
        "explorers_number",
        health.other_number as f64,
        "Number of explorers inspected.",
    )?;
    w.encode_gauge(
        "target_height",
        health.target_height.map(|x| x as f64).unwrap_or(NO_HEIGHT),
        "Target height calculated from the explorers.",
    )?;
    w.encode_gauge(
        "height_diff",
        health
            .height_diff
            .map(|x| x as f64)
            .unwrap_or(NO_HEIGHT_DIFF),
        "Difference between the source and the target heights.",
    )?;

    let (not_enough_data, ok, ahead, behind) = match health.status {
        StatusCode::NotEnoughData => (1.0, 0.0, 0.0, 0.0),
        StatusCode::Ok => (0.0, 1.0, 0.0, 0.0),
        StatusCode::Ahead => (0.0, 0.0, 1.0, 0.0),
        StatusCode::Behind => (0.0, 0.0, 0.0, 1.0),
    };
    w.gauge_vec("status", "Status code of the Bitcoin canister health.")?
        .value(&[("height", "not_enough_data")], not_enough_data)?
        .value(&[("height", "ok")], ok)?
        .value(&[("height", "ahead")], ahead)?
        .value(&[("height", "behind")], behind)?;

    Ok(())
}
