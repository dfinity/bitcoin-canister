use crate::{metrics::InstructionHistogram, state, types::HttpResponse, with_state};
use ic_cdk::api::time;
use ic_metrics_encoder::MetricsEncoder;
use serde_bytes::ByteBuf;
use std::io;

const WASM_PAGE_SIZE: u64 = 65536;

pub fn get_metrics() -> HttpResponse {
    let now = time();
    let mut writer = MetricsEncoder::new(vec![], (now / 1_000_000) as i64);
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
            state.syncing_state.num_get_successors_rejects as f64,
            "The number of rejects received when calling GetSuccessors.",
        )?;
        w.encode_counter(
            "num_block_deserialize_errors",
            state.syncing_state.num_block_deserialize_errors as f64,
            "The number of errors occurred when deserializing blocks.",
        )?;
        w.encode_counter(
            "num_insert_block_errors",
            state.syncing_state.num_insert_block_errors as f64,
            "The number of errors occurred when inserting a block.",
        )?;

        // Profiling
        encode_instruction_histogram(w, &state.metrics.get_utxos_total)?;
        encode_instruction_histogram(w, &state.metrics.get_utxos_apply_unstable_blocks)?;
        encode_instruction_histogram(w, &state.metrics.get_utxos_build_utxos_vec)?;
        encode_instruction_histogram(w, &state.metrics.get_balance_total)?;
        encode_instruction_histogram(w, &state.metrics.get_balance_apply_unstable_blocks)?;
        encode_instruction_histogram(w, &state.metrics.get_current_fee_percentiles_total)?;
        encode_instruction_histogram(w, &state.metrics.block_insertion)?;

        w.encode_gauge(
            "send_transaction_count",
            state.metrics.send_transaction_count as f64,
            "The total number of (valid) requests to the send_transaction endpoint.",
        )?;

        w.encode_gauge(
            "cycles_balance",
            ic_cdk::api::canister_balance() as f64,
            "The cycles balance of the canister.",
        )?;

        encode_labeled_gauge(
            w,
            "block_ingestion_stats",
            "The stats of the most recent block ingested into the stable UTXO set.",
            &state.metrics.block_ingestion_stats.get_labels_and_values(),
        )?;

        Ok(())
    })
}

fn encode_instruction_histogram(
    metrics_encoder: &mut MetricsEncoder<Vec<u8>>,
    h: &InstructionHistogram,
) -> io::Result<()> {
    metrics_encoder.encode_histogram(&h.name, h.buckets(), h.sum, &h.help)
}

fn encode_labeled_gauge(
    metrics_encoder: &mut MetricsEncoder<Vec<u8>>,
    name: &str,
    help: &str,
    labels_and_values: &[((&str, &str), u64)],
) -> io::Result<()> {
    let mut gauge = metrics_encoder.gauge_vec(name, help)?;

    for (label, value) in labels_and_values {
        gauge = gauge.value(&[*label], *value as f64)?;
    }

    Ok(())
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
