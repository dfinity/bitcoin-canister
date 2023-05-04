use crate::health::{HealthStatus, HeightStatus};
use ic_btc_interface::{Config as BitcoinCanisterConfig, Flag};

#[derive(Clone, Debug)]
pub struct ApiAccess {
    /// Expected value of the Bitcoin canister API access flag.
    pub target: Option<Flag>,

    /// Actual value of the Bitcoin canister API access flag.
    pub actual: Option<Flag>,
}

impl ApiAccess {
    pub fn new() -> Self {
        Self {
            target: None,
            actual: None,
        }
    }
}

impl Default for ApiAccess {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculates the target value of the Bitcoin canister API access flag.
fn calculate_target(health: HealthStatus) -> Option<Flag> {
    match health.height_status {
        HeightStatus::Ok => Some(Flag::Enabled),
        HeightStatus::Behind | HeightStatus::Ahead => Some(Flag::Disabled),
        HeightStatus::NotEnoughData => None,
    }
}

async fn get_bitcoin_canister_config() -> Option<BitcoinCanisterConfig> {
    // TODO: read the configuration from the Bitcoin canister.
    Some(BitcoinCanisterConfig::default())
}

/// Fetches the API access flag from the Bitcoin canister.
pub async fn fetch_api_access() {
    let health = crate::health::health_status();
    let target = crate::api_access::calculate_target(health);

    let bitcoin_canister_config = get_bitcoin_canister_config().await;
    let actual = bitcoin_canister_config.map(|config| config.api_access);

    let api_access = ApiAccess { target, actual };

    crate::storage::set_api_access(api_access);
}

/// Sets the API access flag in the Bitcoin canister.
pub async fn set_api_access() {
    let api_access = crate::storage::get_api_access();
    match (api_access.target, api_access.actual) {
        (None, _) => (),
        (Some(target), actual) => {
            if Some(target) != actual {
                // TODO: set the API access in the Bitcoin canister.
            }
        }
    }
}
