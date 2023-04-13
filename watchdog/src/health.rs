use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::fetch::BlockInfo;
use candid::CandidType;

const BLOCKS_BEHIND_THRESHOLD: i64 = -2;
const BLOCKS_AHEAD_THRESHOLD: i64 = 2;
const MIN_EXPLORERS: usize = 3;

#[derive(Clone, Debug, CandidType, PartialEq, Eq)]
pub enum StatusCode {
    Ok,
    Ahead,
    Behind,
    Error(String),
}

#[derive(Clone, Debug, CandidType, PartialEq, Eq)]
pub struct HealthStatus {
    pub source_height: Option<u64>,
    pub target_height: Option<u64>,
    pub height_diff: Option<i64>,
    pub status: StatusCode,
}

/// Calculates the health status.
pub fn calculate() -> HealthStatus {
    compare(
        crate::storage::get(&BitcoinBlockApi::BitcoinCanister),
        BitcoinBlockApi::explorers()
            .iter()
            .filter_map(crate::storage::get)
            .collect::<Vec<_>>(),
    )
}

/// Compares the source with the other explorers.
fn compare(source: Option<BlockInfo>, other: Vec<BlockInfo>) -> HealthStatus {
    let source_height = source.and_then(|block| block.height);
    let heights = other
        .iter()
        .filter_map(|block| block.height)
        .collect::<Vec<_>>();
    let target_height = if heights.len() < MIN_EXPLORERS {
        None // Not enough data from explorers.
    } else {
        median(heights)
    };
    let height_diff = source_height
        .zip(target_height)
        .map(|(source, target)| source as i64 - target as i64);
    let status = height_diff.map_or(StatusCode::Error("Not enough data".to_string()), |diff| {
        if diff < BLOCKS_BEHIND_THRESHOLD {
            StatusCode::Behind
        } else if diff > BLOCKS_AHEAD_THRESHOLD {
            StatusCode::Ahead
        } else {
            StatusCode::Ok
        }
    });

    HealthStatus {
        source_height,
        target_height,
        height_diff,
        status,
    }
}

/// The median of the given values.
fn median<T: std::cmp::Ord + Clone>(mut values: Vec<T>) -> Option<T> {
    let n = values.len();
    if n == 0 {
        None
    } else {
        values.sort();
        let mid = if n % 2 == 0 { (n - 1) / 2 } else { n / 2 };
        Some(values[mid].clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_compare_no_source_neither_explorers() {
        // Arrange
        let source = None;
        let other = vec![];

        // Assert
        assert_eq!(
            compare(source, other),
            HealthStatus {
                source_height: None,
                target_height: None,
                height_diff: None,
                status: StatusCode::Error("Not enough data".to_string()),
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
            compare(source, other),
            HealthStatus {
                source_height: Some(1_000),
                target_height: None,
                height_diff: None,
                status: StatusCode::Error("Not enough data".to_string()),
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
            compare(source, other),
            HealthStatus {
                source_height: Some(1_000),
                target_height: None,
                height_diff: None,
                status: StatusCode::Error("Not enough data".to_string()),
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
            compare(source, other),
            HealthStatus {
                source_height: Some(1_000),
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
            compare(source, other),
            HealthStatus {
                source_height: Some(1_000),
                target_height: Some(995),
                height_diff: Some(5),
                status: StatusCode::Ahead,
            }
        );
    }
}
