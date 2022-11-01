use ic_btc_canister::types::HttpResponse;
use ic_cdk::api::time;
use serde_bytes::ByteBuf;
use std::{io, fmt::Display};

pub fn handle_metrics_request() -> HttpResponse {
    let now = time();
    let mut writer = MetricsEncoder::new(vec![], now / 1_000_000);
    match encode_metrics(&mut writer) {
        Ok(()) => {
            let body = writer.into_inner();
            HttpResponse {
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
        Err(err) => HttpResponse {
            status_code: 500,
            headers: vec![],
            body: ByteBuf::from(format!("Failed to encode metrics: {}", err)),
        },
    }
}

fn encode_metrics(w: &mut MetricsEncoder<Vec<u8>>) -> std::io::Result<()> {
    ic_btc_canister::with_state(|state| {
        w.encode_gauge(
            "main_chain_height",
            ic_btc_canister::state::main_chain_height(state) as f64,
            "Height of the main chain.",
        )?;
        w.encode_gauge(
            "stable_height",
            state.stable_height() as f64,
            "The height of the latest stable block.",
        )?;
        w.encode_gauge(
            "utxos_length",
            state.utxos.utxos_len() as f64,
            "The number of UTXOs in the set.",
        )?;
        w.encode_gauge(
            "address_utxos_length",
            state.utxos.address_utxos_len() as f64,
            "The number of UTXOs that are owned by supported addresses.",
        )?;
        w.encode_counter(
            "num_get_successors_rejects",
            state.syncing_state.num_get_successors_rejects,
            "The number of times an error is received when calling GetSuccessors.",
        )?;
        Ok(())
    })
}

// `MetricsEncoder` provides methods to encode metrics in a text format
// that can be understood by Prometheus.
//
// Metrics are encoded with the block time included, to allow Prometheus
// to discard out-of-order samples collected from replicas that are behind.
//
// See [Exposition Formats][1] for an informal specification of the text format.
//
// [1]: https://github.com/prometheus/docs/blob/master/content/docs/instrumenting/exposition_formats.md
struct MetricsEncoder<W: io::Write> {
    writer: W,
    now_millis: u64,
}

impl<W: io::Write> MetricsEncoder<W> {
    /// Constructs a new encoder dumping metrics with the given timestamp into
    /// the specified writer.
    fn new(writer: W, now_millis: u64) -> Self {
        Self { writer, now_millis }
    }

    /// Returns the internal buffer that was used to record the
    /// metrics.
    fn into_inner(self) -> W {
        self.writer
    }

    fn encode_header(&mut self, name: &str, help: &str, typ: &str) -> io::Result<()> {
        writeln!(self.writer, "# HELP {} {}", name, help)?;
        writeln!(self.writer, "# TYPE {} {}", name, typ)
    }

    fn encode_single_value<T: Display>(
        &mut self,
        typ: &str,
        name: &str,
        value: T,
        help: &str,
    ) -> io::Result<()> {
        self.encode_header(name, help, typ)?;
        writeln!(self.writer, "{} {} {}", name, value, self.now_millis)
    }

    /// Encodes the metadata and the value of a gauge.
    fn encode_gauge(&mut self, name: &str, value: f64, help: &str) -> io::Result<()> {
        self.encode_single_value("gauge", name, value, help)
    }

    fn encode_counter(&mut self, name: &str, value: u64, help: &str)-> io::Result<()> {
        self.encode_single_value("counter", name, value, help)
    }
}
