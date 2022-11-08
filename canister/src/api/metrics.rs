use crate::{state, types::HttpResponse, with_state};
use ic_cdk::api::time;
use serde_bytes::ByteBuf;
use std::{fmt::Display, io};

const WASM_PAGE_SIZE: u64 = 65536;

pub fn get_metrics() -> HttpResponse {
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
    with_state(|state| {
        w.encode_gauge(
            "main_chain_height",
            state::main_chain_height(state) as f64,
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
        w.encode_gauge(
            "stable_memory_size_in_bytes",
            (ic_cdk::api::stable::stable64_size() * WASM_PAGE_SIZE) as f64,
            "The size of stable memory in pages.",
        )?;
        w.encode_gauge(
            "heap_size_in_bytes",
            get_heap_size() as f64,
            "The size of the heap memory in pages.",
        )?;
        w.encode_counter(
            "num_get_successors_rejects",
            state.syncing_state.num_get_successors_rejects,
            "The number of rejects received when calling GetSuccessors.",
        )?;
        w.encode_counter(
            "num_block_deserialize_errors",
            state.syncing_state.num_block_deserialize_errors,
            "The number of errors occurred when deserializing blocks.",
        )?;
        w.encode_counter(
            "num_insert_block_errors",
            state.syncing_state.num_insert_block_errors,
            "The number of errors occurred when inserting a block.",
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

    fn encode_counter(&mut self, name: &str, value: u64, help: &str) -> io::Result<()> {
        self.encode_single_value("counter", name, value, help)
    }
}

// Returns the size of the heap in pages.
fn get_heap_size() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        core::arch::wasm32::memory_size(0) as u64 * WASM_PAGE_SIZE
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}
