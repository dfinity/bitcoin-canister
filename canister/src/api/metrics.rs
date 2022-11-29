use crate::{metrics::InstructionHistogram, state, types::HttpResponse, with_state};
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

        // Memory
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

        // Errors
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

        // Profiling
        w.encode_instruction_histogram(&state.metrics.get_utxos_total)?;
        w.encode_instruction_histogram(&state.metrics.get_utxos_apply_unstable_blocks)?;
        w.encode_instruction_histogram(&state.metrics.get_utxos_build_utxos_vec)?;
        w.encode_instruction_histogram(&state.metrics.get_balance_total)?;
        w.encode_instruction_histogram(&state.metrics.get_balance_apply_unstable_blocks)?;
        w.encode_instruction_histogram(&state.metrics.get_current_fee_percentiles_total)?;

        w.encode_gauge(
            "send_transaction_count",
            state.metrics.send_transaction_count as f64,
            "The total number of (valid) requests to the send_transaction endpoint.",
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

    /// Encodes the metadata and the value of a histogram.
    ///
    /// SUM is the sum of all observed values, before they were put
    /// into buckets.
    ///
    /// BUCKETS is a list (key, value) pairs, where KEY is the bucket
    /// and VALUE is the number of items *in* this bucket (i.e., it's
    /// not a cumulative value).
    pub fn encode_histogram(
        &mut self,
        name: &str,
        buckets: impl Iterator<Item = (f64, f64)>,
        sum: f64,
        help: &str,
    ) -> io::Result<()> {
        self.encode_header(name, help, "histogram")?;
        let mut total: f64 = 0.0;
        let mut saw_infinity = false;
        for (bucket, v) in buckets {
            total += v;
            if bucket == std::f64::INFINITY {
                saw_infinity = true;
                writeln!(
                    self.writer,
                    "{}_bucket{{le=\"+Inf\"}} {} {}",
                    name, total, self.now_millis
                )?;
            } else {
                writeln!(
                    self.writer,
                    "{}_bucket{{le=\"{}\"}} {} {}",
                    name, bucket, total, self.now_millis
                )?;
            }
        }
        if !saw_infinity {
            writeln!(
                self.writer,
                "{}_bucket{{le=\"+Inf\"}} {} {}",
                name, total, self.now_millis
            )?;
        }
        writeln!(self.writer, "{}_sum {} {}", name, sum, self.now_millis)?;
        writeln!(self.writer, "{}_count {} {}", name, total, self.now_millis)
    }

    /// Encodes an `InstructionHistogram`.
    pub fn encode_instruction_histogram(&mut self, h: &InstructionHistogram) -> io::Result<()> {
        self.encode_histogram(&h.name, h.buckets(), h.sum, &h.help)
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
