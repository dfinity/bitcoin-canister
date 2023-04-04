use crate::config::Config;
use crate::types::BlockHeight;
use candid::CandidType;
use serde::{Deserialize, Serialize};

/// The status of the bitcoin_canister.
#[derive(Debug, PartialEq, Serialize, Deserialize, CandidType, Clone)]
enum StatusCode {
    #[serde(rename = "undefined")]
    Undefined = 0,
    #[serde(rename = "ok")]
    Ok = 1,
    #[serde(rename = "behind")]
    Behind = 2,
    #[serde(rename = "ahead")]
    Ahead = 3,
    #[serde(rename = "no_bitcoin_canister_data")]
    NoBitcoinCanisterData = 4,
    #[serde(rename = "not_enough_explorers")]
    NotEnoughExplorers = 5,
    #[serde(rename = "no_pivot_data")]
    NoPivotData = 6,
}

impl StatusCode {
    /// The message of the status code.
    fn message(&self) -> &'static str {
        match self {
            StatusCode::Undefined => "Undefined",
            StatusCode::Ok => "Bitcoin canister block height is within the limits",
            StatusCode::Behind => "Bitcoin canister block height is behind the pivot",
            StatusCode::Ahead => "Bitcoin canister block height is ahead of the pivot",
            StatusCode::NoBitcoinCanisterData => "No bitcoin_canister data",
            StatusCode::NotEnoughExplorers => "Not enough explorers",
            StatusCode::NoPivotData => "No pivot data",
        }
    }
}

/// The status of the bitcoin_canister.
#[derive(Debug, Serialize, Deserialize, CandidType, Clone)]
struct Status {
    code: StatusCode,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    height_diff: Option<i32>,
}

impl Status {
    pub fn new(code: StatusCode, height_diff: Option<i32>) -> Self {
        let message = code.message().to_string();
        Self {
            code,
            message,
            height_diff,
        }
    }
}

/// The information of the watchdog canister.
#[derive(Debug, Serialize, Deserialize, CandidType, Clone)]
pub struct Info {
    config: Config,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    bitcoin_canister: Option<BlockHeight>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pivot: Option<(String, BlockHeight)>,
    explorers_n: u32,
    explorers: Vec<(String, BlockHeight)>,
}

impl Info {
    pub fn new(
        config: Config,
        bitcoin_canister: Option<BlockHeight>,
        explorers: Vec<(String, BlockHeight)>,
    ) -> Self {
        let explorers_n = explorers.len() as u32;
        let info = Self {
            config,
            status: Status::new(StatusCode::Undefined, None),
            bitcoin_canister,
            pivot: median(explorers.clone()),
            explorers_n,
            explorers,
        };

        // Compute the status based on the given data (bitcoin_canister, explorers, pivot).
        let status = match bitcoin_canister {
            // No bitcoin_canister data.
            None => Status::new(StatusCode::NoBitcoinCanisterData, None),
            Some(bitcoin_canister_height) => {
                if info.explorers_n < info.config.min_explorers {
                    // Not enough explorers.
                    Status::new(StatusCode::NotEnoughExplorers, None)
                } else {
                    match &info.pivot {
                        // No pivot data.
                        None => Status::new(StatusCode::NoPivotData, None),
                        Some((_name, pivot)) => {
                            // All data is available.
                            // Compute the difference between the bitcoin_canister height and the pivot.
                            let diff = bitcoin_canister_height.get() as i32 - pivot.get() as i32;
                            if diff < info.config.blocks_behind_threshold {
                                Status::new(StatusCode::Behind, Some(diff))
                            } else if diff > info.config.blocks_ahead_threshold {
                                Status::new(StatusCode::Ahead, Some(diff))
                            } else {
                                Status::new(StatusCode::Ok, Some(diff))
                            }
                        }
                    }
                }
            }
        };

        Self { status, ..info }
    }

    pub fn as_json_str(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
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
    fn test_median() {
        assert_eq!(median::<u64>(vec![]), None);
        assert_eq!(median(vec![1]), Some(1));
        assert_eq!(median(vec![2, 1]), Some(1));
        assert_eq!(median(vec![3, 2, 1]), Some(2));
        assert_eq!(median(vec![4, 3, 2, 1]), Some(2));
        assert_eq!(median(vec![5, 4, 3, 2, 1]), Some(3));

        assert_eq!(
            median(vec![
                BlockHeight::new(3),
                BlockHeight::new(2),
                BlockHeight::new(1)
            ]),
            Some(BlockHeight::new(2))
        );

        assert_eq!(
            median(vec![
                ("ccc", BlockHeight::new(3)),
                ("bbb", BlockHeight::new(2)),
                ("aaa", BlockHeight::new(1)),
            ]),
            Some(("bbb", BlockHeight::new(2)))
        );

        assert_eq!(
            median(vec![
                ("ccc", BlockHeight::new(2)),
                ("bbb", BlockHeight::new(2)),
                ("aaa", BlockHeight::new(2)),
            ]),
            Some(("bbb", BlockHeight::new(2)))
        );

        assert_eq!(
            median(vec![
                ("ccc".to_string(), BlockHeight::new(2)),
                ("bbb".to_string(), BlockHeight::new(2)),
                ("aaa".to_string(), BlockHeight::new(2)),
            ]),
            Some(("bbb".to_string(), BlockHeight::new(2)))
        );
    }

    #[test]
    fn test_info_status_ok() {
        let config = Config::default();
        let bitcoin_canister_height = BlockHeight::new(9);
        let explorers = vec![
            ("ccc".to_string(), BlockHeight::new(12)),
            ("bbb".to_string(), BlockHeight::new(11)),
            ("aaa".to_string(), BlockHeight::new(10)),
        ];

        let info = Info::new(config, Some(bitcoin_canister_height), explorers);

        assert_eq!(info.status.code, StatusCode::Ok);
    }

    #[test]
    fn test_info_status_no_bitcoin_canister_data() {
        let config = Config::default();
        let explorers = vec![
            ("ccc".to_string(), BlockHeight::new(12)),
            ("bbb".to_string(), BlockHeight::new(11)),
            ("aaa".to_string(), BlockHeight::new(10)),
        ];

        let info = Info::new(config, None, explorers);

        assert_eq!(info.status.code, StatusCode::NoBitcoinCanisterData);
    }

    #[test]
    fn test_info_status_not_enough_explorers() {
        let config = Config::default();
        let bitcoin_canister_height = BlockHeight::new(11);
        let explorers = vec![("aaa".to_string(), BlockHeight::new(10))];

        let info = Info::new(config, Some(bitcoin_canister_height), explorers);

        assert_eq!(info.status.code, StatusCode::NotEnoughExplorers);
    }

    #[test]
    fn test_info_status_ahead() {
        let config = Config::default();
        let bitcoin_canister_height = BlockHeight::new(15);
        let explorers = vec![
            ("ccc".to_string(), BlockHeight::new(12)),
            ("bbb".to_string(), BlockHeight::new(11)),
            ("aaa".to_string(), BlockHeight::new(10)),
        ];

        let info = Info::new(config, Some(bitcoin_canister_height), explorers);

        assert_eq!(info.status.code, StatusCode::Ahead);
        assert_eq!(info.status.height_diff, Some(4));
    }

    #[test]
    fn test_info_status_behind() {
        let config = Config::default();
        let bitcoin_canister_height = BlockHeight::new(5);
        let explorers = vec![
            ("ccc".to_string(), BlockHeight::new(12)),
            ("bbb".to_string(), BlockHeight::new(11)),
            ("aaa".to_string(), BlockHeight::new(10)),
        ];

        let info = Info::new(config, Some(bitcoin_canister_height), explorers);

        assert_eq!(info.status.code, StatusCode::Behind);
        assert_eq!(info.status.height_diff, Some(-6));
    }

    #[test]
    fn test_info_status_no_pivot_data() {
        let config = Config {
            min_explorers: 0,
            ..Config::default()
        };
        let bitcoin_canister_height = BlockHeight::new(5);
        let explorers = vec![];

        let info = Info::new(config, Some(bitcoin_canister_height), explorers);

        assert_eq!(info.status.code, StatusCode::NoPivotData);
    }
}
