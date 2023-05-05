use crate::health::{HealthStatus, HeightStatus};
use crate::print;
use candid::CandidType;
use ic_btc_interface::{Config as BitcoinCanisterConfig, Flag, SetConfigRequest};

/// Captures the expected and the actual value of the Bitcoin canister API access flag.
#[derive(Clone, Debug, CandidType)]
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

    /// Checks if the target and actual API access flags are in sync.
    pub fn is_in_sync(&self) -> bool {
        self.target == self.actual
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

/// Fetches the Bitcoin canister config.
async fn get_bitcoin_canister_config() -> Option<BitcoinCanisterConfig> {
    let id = crate::storage::get_config().bitcoin_canister_principal;
    let result = ic_cdk::api::call::call(id, "get_config", ()).await;
    result
        .map(|(config,)| config)
        .map_err(|err| print(&format!("Error getting Bitcoin canister config: {:?}", err)))
        .ok()
}

/// Fetches the actual API access flag and calculates the target value.
async fn fetch_actual_and_calculate_target_api_access() {
    let target = calculate_target(crate::health::health_status());

    let bitcoin_canister_config = get_bitcoin_canister_config().await;
    let actual = bitcoin_canister_config.map(|config| config.api_access);
    if actual.is_none() {
        print("Error getting Bitcoin canister config: api_access is None");
    }

    crate::storage::set_api_access(ApiAccess { target, actual });
}

/// Updates the API access flag in the Bitcoin canister.
async fn update_api_access(target: Option<Flag>) {
    let id = crate::storage::get_config().bitcoin_canister_principal;
    let set_config_request = SetConfigRequest {
        api_access: target,
        ..Default::default()
    };
    let result = ic_cdk::api::call::call(id, "set_config", (set_config_request,)).await;
    match result {
        Ok(()) => (),
        Err(err) => {
            print(&format!("Error setting Bitcoin canister config: {:?}", err));
        }
    }
}

/// Synchronizes the API access flag of the Bitcoin canister, attempting a limited number of times.
pub async fn synchronise_api_access() {
    const ATTEMPTS: u8 = 3;
    for _ in 0..ATTEMPTS {
        fetch_actual_and_calculate_target_api_access().await;
        let api_access = crate::storage::get_api_access();
        if api_access.target.is_none() || api_access.is_in_sync() {
            return;
        }
        update_api_access(api_access.target).await;
    }
    print(&format!(
        "Error: Unable to synchronize API access after {ATTEMPTS:?} attempts."
    ));
}
