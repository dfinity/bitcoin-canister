use serde::{Deserialize, Serialize};

use crate::utxo_set::BlockIngestionStats;

const M: u64 = 1_000_000;
const BUCKET_SIZE: u64 = 500 * M;
const NUM_BUCKETS: u64 = 21;

/// Metrics for various endpoints.
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Metrics {
    pub get_utxos_total: InstructionHistogram,
    pub get_utxos_apply_unstable_blocks: InstructionHistogram,
    pub get_utxos_build_utxos_vec: InstructionHistogram,

    #[serde(default = "default_get_block_headers_total")]
    pub get_block_headers_total: InstructionHistogram,
    #[serde(default = "default_get_block_headers_stable_blocks")]
    pub get_block_headers_stable_blocks: InstructionHistogram,
    #[serde(default = "default_get_block_headers_unstable_blocks")]
    pub get_block_headers_unstable_blocks: InstructionHistogram,

    pub get_balance_total: InstructionHistogram,
    pub get_balance_apply_unstable_blocks: InstructionHistogram,

    pub get_current_fee_percentiles_total: InstructionHistogram,

    /// The total number of (valid) requests sent to `send_transaction`.
    pub send_transaction_count: u64,

    /// The stats of the most recent block ingested into the stable UTXO set.
    pub block_ingestion_stats: BlockIngestionStats,

    /// Instructions needed to insert a block into the pool of unstable blocks.
    pub block_insertion: InstructionHistogram,

    /// The total number of cycles burnt.
    pub cycles_burnt: Option<u128>,

    /// The time interval between two consecutive GetSuccessors requests.
    #[serde(default = "get_successors_request_interval")]
    pub get_successors_request_interval: Histogram,

    /// The depth distribution of unstable block branches.
    #[serde(default = "unstable_blocks_tip_depths")]
    pub unstable_blocks_tip_depths: Histogram,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            get_utxos_total: InstructionHistogram::new(
                "ins_get_utxos_total",
                "Instructions needed to execute a get_utxos request.",
            ),
            get_utxos_apply_unstable_blocks: InstructionHistogram::new(
                "ins_get_utxos_apply_unstable_blocks",
                "Instructions needed to apply the unstable blocks in a get_utxos request.",
            ),
            get_utxos_build_utxos_vec: InstructionHistogram::new(
                "inst_count_get_utxos_build_utxos_vec",
                "Instructions needed to build the UTXOs vec in a get_utxos request.",
            ),

            get_block_headers_total: default_get_block_headers_total(),
            get_block_headers_stable_blocks: default_get_block_headers_stable_blocks(),
            get_block_headers_unstable_blocks: default_get_block_headers_unstable_blocks(),

            get_balance_total: InstructionHistogram::new(
                "ins_get_balance_total",
                "Instructions needed to execute a get_balance request.",
            ),
            get_balance_apply_unstable_blocks: InstructionHistogram::new(
                "ins_get_balance_apply_unstable_blocks",
                "Instructions needed to apply the unstable blocks in a get_utxos request.",
            ),

            get_current_fee_percentiles_total: InstructionHistogram::new(
                "ins_get_current_fee_percentiles_total",
                "Instructions needed to execute a get_current_fee_percentiles request.",
            ),

            send_transaction_count: 0,

            block_ingestion_stats: BlockIngestionStats::default(),

            block_insertion: InstructionHistogram::new(
                "ins_block_insertion",
                "Instructions needed to insert a block into the pool of unstable blocks.",
            ),

            cycles_burnt: Some(0),

            get_successors_request_interval: get_successors_request_interval(),

            unstable_blocks_tip_depths: unstable_blocks_tip_depths(),
        }
    }
}

/// A histogram for observing instruction counts.
///
/// The histogram observes the values in buckets of:
///
///  (500M, 1B, 1.5B, ..., 9B, 9.5B, 10B, +Inf)
#[derive(Serialize, Deserialize, PartialEq)]
pub struct InstructionHistogram {
    pub name: String,
    pub buckets: Vec<u64>,
    pub sum: f64,
    pub help: String,
}

impl InstructionHistogram {
    pub fn new<S: Into<String>>(name: S, help: S) -> Self {
        Self {
            name: name.into(),
            help: help.into(),
            sum: 0.0,
            buckets: vec![0; 21],
        }
    }

    /// Observes an instruction count.
    pub fn observe(&mut self, value: u64) {
        let bucket_idx = Self::get_bucket(value);

        // Divide value by 1M to keep the counts sane.
        let value: f64 = value as f64 / M as f64;

        self.buckets[bucket_idx] += 1;

        self.sum += value;
    }

    /// Returns an iterator with the various buckets.
    pub fn buckets(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        (500..10_500)
            .step_by((BUCKET_SIZE / M) as usize)
            .map(|e| e as f64)
            .chain([f64::INFINITY])
            .zip(self.buckets.iter().map(|e| *e as f64))
    }

    // Returns the index of the bucket where the value belongs.
    fn get_bucket(value: u64) -> usize {
        if value == 0 {
            return 0;
        }

        let idx = (value - 1) / BUCKET_SIZE;
        std::cmp::min(idx, NUM_BUCKETS - 1) as usize
    }
}

fn default_get_block_headers_total() -> InstructionHistogram {
    InstructionHistogram::new(
        "ins_block_headers_total",
        "Instructions needed to execute a get_block_headers request.",
    )
}

fn default_get_block_headers_stable_blocks() -> InstructionHistogram {
    InstructionHistogram::new(
        "inst_count_get_block_headers_stable_blocks",
        "Instructions needed to build the block headers vec in a get_block_headers request from stable blocks.",
    )
}

fn default_get_block_headers_unstable_blocks() -> InstructionHistogram {
    InstructionHistogram::new(
        "inst_count_get_block_headers_unstable_blocks",
        "Instructions needed to build the block headers vec in a get_block_headers request from unstable blocks.",
    )
}

/// A histogram for observing values, using custom bucket thresholds.
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Histogram {
    pub name: String,
    pub help: String,
    thresholds: Vec<u64>,  // Stores bucket thresholds
    pub buckets: Vec<u64>, // Stores observation counts per bucket
    pub sum: f64,
}

impl Histogram {
    /// Creates a new histogram with custom bucket thresholds.
    ///
    /// `thresholds` must be a sorted vector of bucket limits (e.g., from `decimal_buckets()`).
    pub fn new<S: Into<String>>(name: S, help: S, thresholds: Vec<u64>) -> Self {
        assert!(
            thresholds.windows(2).all(|w| w[0] < w[1]),
            "Thresholds must be strictly increasing"
        );

        Self {
            name: name.into(),
            help: help.into(),
            sum: 0.0,
            buckets: vec![0; thresholds.len()], // One count per threshold
            thresholds,
        }
    }

    /// Observes a new value by updating the corresponding bucket count.
    pub fn observe(&mut self, value: f64) {
        if value < 0.0 {
            return; // Ignore negative values
        }
        let bucket_idx = self.get_bucket(value);
        self.buckets[bucket_idx] += 1;
        self.sum += value;
    }

    /// Finds the index of the bucket where `value` belongs.
    fn get_bucket(&self, value: f64) -> usize {
        let value = value as u64;
        match self.thresholds.binary_search(&value) {
            Ok(idx) => idx,                              // Exact match found
            Err(idx) => idx.min(self.buckets.len() - 1), // Next larger bucket or last bucket
        }
    }

    /// Returns an iterator over bucket thresholds and their observed counts.
    pub fn buckets(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.thresholds
            .iter()
            .map(|&e| e as f64)
            .chain(std::iter::once(f64::INFINITY))
            .zip(self.buckets.iter().map(|&e| e as f64))
    }
}

/// Generates logarithmic bucket thresholds using powers of 10 with `[1, 2, 5]` steps.
///
/// Example: `logarithmic_buckets(0, 4)` → `[1, 2, 5, 10, ..., 50_000]`
///
/// # Panics
/// Panics if `min_power > max_power`.
fn logarithmic_buckets(min_power: u32, max_power: u32) -> Vec<u64> {
    assert!(
        min_power <= max_power,
        "min_power must be <= max_power, given {} and {}",
        min_power,
        max_power
    );
    let mut buckets = Vec::with_capacity(3 * (max_power - min_power + 1) as usize);
    for n in min_power..=max_power {
        for &m in &[1, 2, 5] {
            buckets.push(m * 10_u64.pow(n));
        }
    }
    buckets
}

fn get_successors_request_interval() -> Histogram {
    Histogram::new(
        "get_successors_request_interval",
        "Time between consecutive GetSuccessors requests (1s to 50,000s).",
        logarithmic_buckets(0, 4),
    )
}

fn unstable_blocks_tip_depths() -> Histogram {
    Histogram::new(
        "unstable_blocks_tip_depths",
        "Depth distribution of unstable block branches (1 to 50,000).",
        logarithmic_buckets(0, 4),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_buckets() {
        let h = InstructionHistogram::new("", "");
        assert_eq!(
            h.buckets().collect::<Vec<_>>(),
            vec![
                (500.0, 0.0),
                (1000.0, 0.0),
                (1500.0, 0.0),
                (2000.0, 0.0),
                (2500.0, 0.0),
                (3000.0, 0.0),
                (3500.0, 0.0),
                (4000.0, 0.0),
                (4500.0, 0.0),
                (5000.0, 0.0),
                (5500.0, 0.0),
                (6000.0, 0.0),
                (6500.0, 0.0),
                (7000.0, 0.0),
                (7500.0, 0.0),
                (8000.0, 0.0),
                (8500.0, 0.0),
                (9000.0, 0.0),
                (9500.0, 0.0),
                (10000.0, 0.0),
                (f64::INFINITY, 0.0),
            ]
        );
        assert_eq!(h.sum, 0.0);
    }

    #[test]
    fn observing_values() {
        let mut h = InstructionHistogram::new("", "");
        h.observe(500 * M);
        assert_eq!(
            h.buckets().take(3).collect::<Vec<_>>(),
            vec![(500.0, 1.0), (1000.0, 0.0), (1500.0, 0.0)]
        );
        assert_eq!(h.sum, 500_f64);

        h.observe(1);
        assert_eq!(
            h.buckets().take(3).collect::<Vec<_>>(),
            vec![(500.0, 2.0), (1000.0, 0.0), (1500.0, 0.0)]
        );
        assert_eq!(h.sum, 500.000001);

        h.observe(500 * M + 1);
        assert_eq!(
            h.buckets().take(3).collect::<Vec<_>>(),
            vec![(500.0, 2.0), (1000.0, 1.0), (1500.0, 0.0)]
        );
        assert_eq!(h.sum, 1000.000002);

        h.observe(0);
        assert_eq!(
            h.buckets().take(3).collect::<Vec<_>>(),
            vec![(500.0, 3.0), (1000.0, 1.0), (1500.0, 0.0)]
        );
        assert_eq!(h.sum, 1000.000002);
    }

    #[test]
    fn infinity_bucket() {
        let mut h = InstructionHistogram::new("", "");
        h.observe(10_000 * M + 1);
        assert_eq!(
            h.buckets().skip(20).collect::<Vec<_>>(),
            vec![(f64::INFINITY, 1.0)]
        );
        assert_eq!(h.sum, 10_000.000001);

        h.observe(u64::MAX);
        assert_eq!(
            h.buckets().skip(20).collect::<Vec<_>>(),
            vec![(f64::INFINITY, 2.0)]
        );
    }
}
