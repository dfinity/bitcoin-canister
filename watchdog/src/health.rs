use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::config::Config;
use crate::fetch::BlockInfo;
use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Bitcoin canister height status compared to other explorers.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeightStatus {
    /// Not enough data to calculate the status.
    #[serde(rename = "not_enough_data")]
    NotEnoughData,

    /// Bitcoin canister height is healthy.
    #[serde(rename = "ok")]
    Ok,

    /// Bitcoin canister height is ahead of other explorers, might not be healthy.
    #[serde(rename = "ahead")]
    Ahead,

    /// Bitcoin canister height is behind other explorers, might not be healthy.
    #[serde(rename = "behind")]
    Behind,
}

/// Health status of the Bitcoin canister.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Main chain height of the Bitcoin canister.
    pub height_source: Option<u64>,

    /// Height target derived from explorer heights.
    pub height_target: Option<u64>,

    /// Difference between Bitcoin canister height and target height.
    pub height_diff: Option<i64>,

    /// Bitcoin canister height status.
    pub height_status: HeightStatus,

    /// Block info from the explorers.
    pub explorers: Vec<BlockInfo>,
}

/// Calculates the health status of a Bitcoin canister.
pub fn health_status() -> HealthStatus {
    let bitcoin_network = crate::storage::get_config().bitcoin_network;
    compare(
        crate::storage::get_block_info(&BitcoinBlockApi::BitcoinCanister),
        BitcoinBlockApi::network_explorers(bitcoin_network)
            .iter()
            .filter_map(crate::storage::get_block_info)
            .collect::<Vec<_>>(),
        crate::storage::get_config(),
    )
}

/// Returns the median if `min_explorers` are within the block range around it.
fn calculate_height_target(
    heights: &[u64],
    min_explorers: usize,
    blocks_behind_threshold: i64,
    blocks_ahead_threshold: i64,
) -> Option<u64> {
    if heights.len() < min_explorers {
        return None;
    }

    let threshold = median(heights)? as i64;
    let (lo, hi) = (
        threshold.saturating_add(blocks_behind_threshold) as u64,
        threshold.saturating_add(blocks_ahead_threshold) as u64,
    );
    let valid_explorers = heights
        .iter()
        .filter(|&height| (lo..=hi).contains(height))
        .count();

    if valid_explorers >= min_explorers {
        Some(threshold as u64)
    } else {
        None
    }
}

/// Compares the source with the other explorers.
fn compare(source: Option<BlockInfo>, explorers: Vec<BlockInfo>, config: Config) -> HealthStatus {
    let height_source = source.and_then(|block| block.height);
    let heights = explorers
        .iter()
        .filter_map(|block| block.height)
        .collect::<Vec<_>>();
    let height_target = calculate_height_target(
        &heights,
        config.min_explorers as usize,
        config.get_blocks_behind_threshold(),
        config.get_blocks_ahead_threshold(),
    );
    let height_diff = height_source
        .zip(height_target)
        .map(|(source, target)| source as i64 - target as i64);
    let height_status = height_diff.map_or(HeightStatus::NotEnoughData, |diff| {
        if diff < config.get_blocks_behind_threshold() {
            HeightStatus::Behind
        } else if diff > config.get_blocks_ahead_threshold() {
            HeightStatus::Ahead
        } else {
            HeightStatus::Ok
        }
    });

    HealthStatus {
        height_source,
        height_target,
        height_diff,
        height_status,
        explorers,
    }
}

/// The median of the given values.
fn median(values: &[u64]) -> Option<u64> {
    let length = values.len();

    if length == 0 {
        return None;
    }

    let mut values = values.to_vec();
    values.sort();

    let mid_index = length / 2;
    let median_value = if length % 2 == 0 {
        (values[mid_index - 1] + values[mid_index]) / 2
    } else {
        values[mid_index]
    };

    Some(median_value)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bitcoin_block_apis::BitcoinBlockApi;

    #[test]
    fn test_median() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[1]), Some(1));
        assert_eq!(median(&[2, 1]), Some(1));
        assert_eq!(median(&[3, 2, 1]), Some(2));
        assert_eq!(median(&[4, 3, 2, 1]), Some(2));
        assert_eq!(median(&[5, 4, 3, 2, 1]), Some(3));
        assert_eq!(median(&[20, 20, 10, 10]), Some(15));
        assert_eq!(median(&[20, 15, 10]), Some(15));
    }

    /// Test data for `calculate_height_target`.
    struct CalculateHeightTargetTestData {
        heights: &'static [u64],
        min_explorers: usize,
        blocks_behind_threshold: i64,
        blocks_ahead_threshold: i64,
        expected: Option<u64>,
    }

    /// Tests `calculate_height_target` with the given test data.
    fn test_calculate_height_target(params: CalculateHeightTargetTestData) {
        assert_eq!(
            calculate_height_target(
                params.heights,
                params.min_explorers,
                params.blocks_behind_threshold,
                params.blocks_ahead_threshold
            ),
            params.expected
        );
    }

    #[test]
    fn test_calculate_height_target_not_enough_explorers() {
        test_calculate_height_target(CalculateHeightTargetTestData {
            heights: &[10],
            min_explorers: 3,
            blocks_behind_threshold: -1,
            blocks_ahead_threshold: 1,
            expected: None,
        });
    }

    #[test]
    fn test_calculate_height_target_explorers_not_in_range() {
        test_calculate_height_target(CalculateHeightTargetTestData {
            heights: &[10, 20, 30],
            min_explorers: 3,
            blocks_behind_threshold: -1,
            blocks_ahead_threshold: 1,
            expected: None,
        });
    }

    #[test]
    fn test_calculate_height_target_explorers_are_in_range() {
        test_calculate_height_target(CalculateHeightTargetTestData {
            heights: &[10, 11, 12],
            min_explorers: 3,
            blocks_behind_threshold: -1,
            blocks_ahead_threshold: 1,
            expected: Some(11),
        });
    }

    #[test]
    fn test_compare_no_source_neither_explorers() {
        // Arrange
        let source = None;
        let other = vec![];

        // Assert
        assert_eq!(
            compare(source, other, crate::storage::get_config()),
            HealthStatus {
                height_source: None,
                height_target: None,
                height_diff: None,
                height_status: HeightStatus::NotEnoughData,
                explorers: vec![],
            }
        );
    }

    #[test]
    fn test_compare_no_explorers() {
        // Arrange
        let source = Some(BlockInfo::new(BitcoinBlockApi::BitcoinCanister, 1_000));
        let other = vec![];

        // Assert
        assert_eq!(
            compare(source, other, crate::storage::get_config()),
            HealthStatus {
                height_source: Some(1_000),
                height_target: None,
                height_diff: None,
                height_status: HeightStatus::NotEnoughData,
                explorers: vec![],
            }
        );
    }

    #[test]
    fn test_compare_2_explorers_are_not_enough() {
        // Arrange
        let source = Some(BlockInfo::new(BitcoinBlockApi::BitcoinCanister, 1_000));
        let other = vec![
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 1_006),
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 1_005),
        ];

        // Assert
        assert_eq!(
            compare(source, other, crate::storage::get_config()),
            HealthStatus {
                height_source: Some(1_000),
                height_target: None,
                height_diff: None,
                height_status: HeightStatus::NotEnoughData,
                explorers: vec![
                    BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 1_006),
                    BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 1_005),
                ],
            }
        );
    }

    #[test]
    fn test_compare_behind() {
        // Arrange
        let source = Some(BlockInfo::new(BitcoinBlockApi::BitcoinCanister, 1_000));
        let other = vec![
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 1_006),
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 1_005),
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 1_004),
        ];

        // Assert
        assert_eq!(
            compare(source, other, crate::storage::get_config()),
            HealthStatus {
                height_source: Some(1_000),
                height_target: Some(1_005),
                height_diff: Some(-5),
                height_status: HeightStatus::Behind,
                explorers: vec![
                    BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 1_006),
                    BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 1_005),
                    BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 1_004),
                ],
            }
        );
    }

    #[test]
    fn test_compare_ahead() {
        // Arrange
        let source = Some(BlockInfo::new(BitcoinBlockApi::BitcoinCanister, 1_000));
        let other = vec![
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 996),
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 995),
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 994),
        ];

        // Assert
        assert_eq!(
            compare(source, other, crate::storage::get_config()),
            HealthStatus {
                height_source: Some(1_000),
                height_target: Some(995),
                height_diff: Some(5),
                height_status: HeightStatus::Ahead,
                explorers: vec![
                    BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 996),
                    BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 995),
                    BlockInfo::new(BitcoinBlockApi::ApiBlockchairComMainnet, 994),
                ],
            }
        );
    }
}
