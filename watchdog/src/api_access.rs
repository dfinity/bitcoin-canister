use crate::health::HealthStatus;
use crate::{health::HeightStatus, print};
use ic_btc_interface::{Config as CanisterConfig, Flag, SetConfigRequest};

/// Calculates the target value of the canister API access flag.
fn calculate_target(health: HealthStatus) -> Option<Flag> {
    match health.height_status {
        HeightStatus::Ok => Some(Flag::Enabled),
        HeightStatus::Behind | HeightStatus::Ahead => Some(Flag::Disabled),
        HeightStatus::NotEnoughData => None,
    }
}

/// Fetches the canister config.
async fn get_target_canister_config() -> Option<CanisterConfig> {
    let id = crate::storage::get_canister().canister_principal();
    let result = ic_cdk::call::Call::unbounded_wait(id, "get_config")
        .with_args(&())
        .await
        .map_err(|err| print(&format!("Error getting canister config: {:?}", err)))
        .ok()?;
    let config = result
        .candid()
        .map_err(|err| print(&format!("Error decoding get_config result: {:?}", err)))
        .ok()?;
    Some(config)
}

/// Fetches the actual API access flag from the canister.
async fn fetch_actual_api_access() -> Option<Flag> {
    let canister_config = get_target_canister_config().await;

    let actual = canister_config.map(|config| config.api_access);
    if actual.is_none() {
        print("Error getting canister config: api_access is None");
    }

    actual
}

/// Updates the API access flag in the canister.
async fn update_api_access(target: Option<Flag>) {
    let id = crate::storage::get_canister().canister_principal();
    let set_config_request = SetConfigRequest {
        api_access: target,
        ..Default::default()
    };
    ic_cdk::call::Call::unbounded_wait(id, "set_config")
        .with_args(&(set_config_request,))
        .await
        .map_err(|err| print(&format!("Error setting canister config: {:?}", err)))
        .ok();
}

/// Synchronizes the API access flag of the canister.
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
