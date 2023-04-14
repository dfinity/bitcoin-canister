/// Below this threshold, the canister is considered to be behind.
pub const BLOCKS_BEHIND_THRESHOLD: i64 = -2;

/// Above this threshold, the canister is considered to be ahead.
pub const BLOCKS_AHEAD_THRESHOLD: i64 = 2;

/// The minimum number of explorers to compare against.
pub const MIN_EXPLORERS: u64 = 3;

/// Bitcoin canister endpoint.
pub const BITCOIN_CANISTER_ENDPOINT: &str =
    "https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics";

/// The number of seconds to wait before the first data fetch.
pub const DELAY_BEFORE_FIRST_FETCH_SEC: u64 = 1;

/// The number of seconds to wait between all the other data fetches.
pub const INTERVAL_BETWEEN_FETCHES_SEC: u64 = 60;
