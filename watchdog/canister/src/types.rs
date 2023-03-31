use candid::CandidType;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(
    Debug, PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Serialize, Deserialize, CandidType,
)]
pub struct BlockHeight(u64);

impl BlockHeight {
    pub fn new(height: u64) -> Self {
        Self(height)
    }

    pub fn get(&self) -> u64 {
        self.0
    }

    pub fn from_json(json: &serde_json::Value) -> Option<Self> {
        json.as_u64().map(Self::new)
    }

    pub fn as_json(&self) -> serde_json::Value {
        json!(self.0)
    }

    pub fn from_string(str: String) -> Option<Self> {
        str.parse::<u64>().map(Self::new).ok()
    }
}
