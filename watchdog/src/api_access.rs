use crate::health::{HealthStatus, HeightStatus};
use crate::print;
use ic_btc_interface::{Config as BitcoinCanisterConfig, Flag, SetConfigRequest};

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

/// Fetches the actual API access flag from the Bitcoin canister.
async fn fetch_actual_api_access() -> Option<Flag> {
    let bitcoin_canister_config = get_bitcoin_canister_config().await;

    let actual = bitcoin_canister_config.map(|config| config.api_access);
    if actual.is_none() {
        print("Error getting Bitcoin canister config: api_access is None");
    }

    actual
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

/// Synchronizes the API access flag of the Bitcoin canister.
pub async fn synchronise_api_access() {
    let target = calculate_target(crate::health::health_status());
    crate::storage::set_api_access_target(target);

    if target.is_some() {
        let actual = fetch_actual_api_access().await;
        if target != actual {
            // Only update the API access flag if the target is not None
            // and it is different from the actual value.
            update_api_access(target).await;
        }
    }
}
