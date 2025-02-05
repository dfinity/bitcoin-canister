use crate::{
    metrics::DurationHistogram, metrics::InstructionHistogram, state, types::HttpResponse,
    with_state,
};
use ic_btc_interface::Flag;
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
        // General stats
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

        // Unstable blocks and stability threshold
        w.encode_gauge(
            "anchor_difficulty",
            state.unstable_blocks.anchor_difficulty() as f64,
            "The difficulty of the anchor block.",
        )?;
        w.encode_gauge(
            "normalized_stability_threshold",
            state.unstable_blocks.normalized_stability_threshold() as f64,
            "The stability threshold normalized by the difficulty of the anchor block.",
        )?;
        w.encode_gauge(
            "unstable_blocks_num_tips",
            state.unstable_blocks.num_tips() as f64,
            "The number of tips in the unstable block tree.",
        )?;
        w.encode_gauge(
            "unstable_blocks_total",
            state::get_unstable_blocks(state).len() as f64,
            "The number of unstable blocks.",
        )?;
        w.encode_gauge(
            "unstable_blocks_depth",
            state.unstable_blocks.blocks_depth() as f64,
            "The depth of the unstable blocks.",
        )?;
        w.encode_gauge(
            "unstable_blocks_difficulty_based_depth",
            state.unstable_blocks.blocks_difficulty_based_depth() as f64,
            "The difficulty-based depth of the unstable blocks.",
        )?;

        // Memory
        w.encode_gauge(
            "stable_memory_size_in_bytes",
            (ic_cdk::api::stable::stable_size() * WASM_PAGE_SIZE) as f64,
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
            "cycles_burnt",
            state.metrics.cycles_burnt.unwrap_or_default() as f64,
            "The total number of cycles burnt.",
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
            &state
                .metrics
                .block_ingestion_stats
                .get_instruction_labels_and_values(),
        )?;

        w.encode_gauge(
            "block_ingestion_num_rounds",
            state.metrics.block_ingestion_stats.get_num_rounds() as f64,
            "The number of rounds it took the most recent block to get ingested into the stable UTXO set.",
        )?;

        w.encode_gauge(
            "is_synced",
            if crate::is_synced() { 1.0 } else { 0.0 },
            "Is the canister synced with the network?",
        )?;

        let (enabled, disabled) = match state.api_access {
            Flag::Enabled => (1.0, 0.0),
            Flag::Disabled => (0.0, 1.0),
        };
        w.gauge_vec(
            "api_access",
            "Flag to control access to the APIs provided by the canister.",
        )?
        .value(&[("flag", "enabled")], enabled)?
        .value(&[("flag", "disabled")], disabled)?;

        encode_labeled_gauge(
            w,
            "get_successors_request_count",
            "The number of get_successors requests.",
            &state
                .syncing_state
                .get_successors_request_stats
                .get_count_metrics(),
        )?;

        encode_duration_histogram(w, &state.metrics.get_successors_request_interval_seconds)?;

        encode_labeled_gauge(
            w,
            "get_successors_response_count",
            "The number of get_successors responses.",
            &state
                .syncing_state
                .get_successors_response_stats
                .get_count_metrics(),
        )?;
        encode_labeled_gauge(
            w,
            "get_successors_response_block_count",
            "The number of blocks in get_successors responses.",
            &state
                .syncing_state
                .get_successors_response_stats
                .get_block_count_metrics(),
        )?;
        encode_labeled_gauge(
            w,
            "get_successors_response_block_size",
            "The total size of the blocks in get_successors responses.",
            &state
                .syncing_state
                .get_successors_response_stats
                .get_block_size_metrics(),
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

fn encode_duration_histogram(
    metrics_encoder: &mut MetricsEncoder<Vec<u8>>,
    h: &DurationHistogram,
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
