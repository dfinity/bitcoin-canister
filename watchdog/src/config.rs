/// The configuration of the watchdog canister.
use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, CandidType)]
pub struct Config {
    pub timer_interval_secs: u32,
    pub min_explorers: u32,
    pub blocks_ahead_threshold: i32,
    pub blocks_behind_threshold: i32,
    pub storage_ttl_millis: u64,
    pub bitcoin_canister_host: String,
}

const ONE_SECOND: u64 = 1_000; // 10^3 milli-seconds in one second.
const ONE_MINUTE: u64 = 60 * ONE_SECOND;

impl Default for Config {
    /// The default configuration.
    fn default() -> Self {
        Self {
            timer_interval_secs: 60,
            min_explorers: 2,
            blocks_ahead_threshold: 2,
            blocks_behind_threshold: -2,
            storage_ttl_millis: 5 * ONE_MINUTE,
            bitcoin_canister_host: "ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app".to_string(),
        }
    }
}
