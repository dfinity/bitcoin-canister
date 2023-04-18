use crate::config::Config;
use crate::fetch::BlockInfo;
use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Status codes for the health of Bitcoin canister compared to other explorers.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusCode {
    /// Not enough data to calculate the health status.
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

/// The health status of the Bitcoin canister.
#[derive(Clone, Debug, CandidType, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    /// The height of the block from the Bitcoin canister.
    pub source_height: Option<u64>,

    /// The number of explorers inspected.
    pub other_number: u64,

    /// The heights of the blocks from the explorers.
    pub other_heights: Vec<u64>,

    /// The target height of the Bitcoin canister calculated
    /// from the explorers.
    pub target_height: Option<u64>,

    /// The difference between the source height
    /// and the target height.
    pub height_diff: Option<i64>,

    /// The code of the health status.
    pub status: StatusCode,
}

/// Compares the source with the other explorers.
pub fn compare(source: Option<BlockInfo>, other: Vec<BlockInfo>, config: Config) -> HealthStatus {
    let source_height = source.and_then(|block| block.height);
    let heights = other
        .iter()
        .filter_map(|block| block.height)
        .collect::<Vec<_>>();
    let other_number = heights.len() as u64;
    let other_heights = heights.clone();
    let target_height = if other_number < config.min_explores {
        None // Not enough data from explorers.
    } else {
        median(heights)
    };
    let height_diff = source_height
        .zip(target_height)
        .map(|(source, target)| source as i64 - target as i64);
    let status = height_diff.map_or(StatusCode::NotEnoughData, |diff| {
        if diff < -(config.blocks_behind_threshold as i64) {
            StatusCode::Behind
        } else if diff > config.blocks_ahead_threshold as i64 {
            StatusCode::Ahead
        } else {
            StatusCode::Ok
        }
    });

    HealthStatus {
        source_height,
        other_number,
        other_heights,
        target_height,
        height_diff,
        status,
    }
}

/// The median of the given values.
fn median(mut values: Vec<u64>) -> Option<u64> {
    let length = values.len();

    if length == 0 {
        return None;
    }

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
        assert_eq!(median(vec![]), None);
        assert_eq!(median(vec![1]), Some(1));
        assert_eq!(median(vec![2, 1]), Some(1));
        assert_eq!(median(vec![3, 2, 1]), Some(2));
        assert_eq!(median(vec![4, 3, 2, 1]), Some(2));
        assert_eq!(median(vec![5, 4, 3, 2, 1]), Some(3));
        assert_eq!(median(vec![20, 20, 10, 10]), Some(15));
        assert_eq!(median(vec![20, 15, 10]), Some(15));
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
                source_height: None,
                other_number: 0,
                other_heights: vec![],
                target_height: None,
                height_diff: None,
                status: StatusCode::NotEnoughData,
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
                source_height: Some(1_000),
                other_number: 0,
                other_heights: vec![],
                target_height: None,
                height_diff: None,
                status: StatusCode::NotEnoughData,
            }
        );
    }

    #[test]
    fn test_compare_not_enough_explorers() {
        // Arrange
        let source = Some(BlockInfo::new(BitcoinBlockApi::BitcoinCanister, 1_000));
        let other = vec![
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairCom, 1_005),
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairCom, 1_005),
        ];

        // Assert
        assert_eq!(
            compare(source, other, crate::storage::get_config()),
            HealthStatus {
                source_height: Some(1_000),
                other_number: 2,
                other_heights: vec![1_005, 1_005],
                target_height: None,
                height_diff: None,
                status: StatusCode::NotEnoughData,
            }
        );
    }

    #[test]
    fn test_compare_behind() {
        // Arrange
        let source = Some(BlockInfo::new(BitcoinBlockApi::BitcoinCanister, 1_000));
        let other = vec![
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairCom, 1_006),
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairCom, 1_005),
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairCom, 1_004),
        ];

        // Assert
        assert_eq!(
            compare(source, other, crate::storage::get_config()),
            HealthStatus {
                source_height: Some(1_000),
                other_number: 3,
                other_heights: vec![1_006, 1_005, 1_004],
                target_height: Some(1_005),
                height_diff: Some(-5),
                status: StatusCode::Behind,
            }
        );
    }

    #[test]
    fn test_compare_ahead() {
        // Arrange
        let source = Some(BlockInfo::new(BitcoinBlockApi::BitcoinCanister, 1_000));
        let other = vec![
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairCom, 996),
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairCom, 995),
            BlockInfo::new(BitcoinBlockApi::ApiBlockchairCom, 994),
        ];

        // Assert
        assert_eq!(
            compare(source, other, crate::storage::get_config()),
            HealthStatus {
                source_height: Some(1_000),
                other_number: 3,
                other_heights: vec![996, 995, 994],
                target_height: Some(995),
                height_diff: Some(5),
                status: StatusCode::Ahead,
            }
        );
    }
}
