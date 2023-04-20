use crate::{MOCK_CALLER, MOCK_CONTROLLERS};
use candid::Principal;
use ic_btc_interface::SetConfigRequest;
use std::convert::TryInto;

pub async fn set_config(request: SetConfigRequest) {
    if is_watchdog_caller() {
        // The watchdog canister can only set the API access flag.
        set_api_access(request);
    } else {
        verify_caller().await;
        set_config_no_verification(request);
    }
}

fn is_watchdog_caller() -> bool {
    crate::with_state(|s| Some(caller()) == s.watchdog_canister)
}

fn set_api_access(request: SetConfigRequest) {
    crate::with_state_mut(|s| {
        if let Some(api_access) = request.api_access {
            s.api_access = api_access;
        }
    });
}

fn set_config_no_verification(request: SetConfigRequest) {
    crate::with_state_mut(|s| {
        if let Some(syncing) = request.syncing {
            s.syncing_state.syncing = syncing;
        }

        if let Some(fees) = request.fees {
            s.fees = fees;
        }

        if let Some(stability_threshold) = request.stability_threshold {
            s.unstable_blocks.set_stability_threshold(
                stability_threshold
                    .try_into()
                    .expect("stability threshold too large"),
            );
        }

        if let Some(api_access) = request.api_access {
            s.api_access = api_access;
        }
        if let Some(disable_api_if_not_fully_synced) = request.disable_api_if_not_fully_synced {
            s.disable_api_if_not_fully_synced = disable_api_if_not_fully_synced;
        }
    });
}

async fn verify_caller() {
    let caller = caller();
    let controllers = controllers().await;
    if !controllers.contains(&caller) {
        panic!("Only controllers can call set_config");
    }
}

fn caller() -> Principal {
    #[cfg(not(target_arch = "wasm32"))]
    {
        MOCK_CALLER.with(|cell| cell.borrow().unwrap_or(Principal::anonymous()))
    }

    #[cfg(target_arch = "wasm32")]
    {
        ic_cdk::caller()
    }
}

async fn controllers() -> Vec<Principal> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        MOCK_CONTROLLERS.with(|cell| cell.borrow().clone().unwrap_or_default())
    }

    #[cfg(target_arch = "wasm32")]
    {
        ic_cdk::api::management_canister::main::canister_status(
            ic_cdk::api::management_canister::main::CanisterIdRecord {
                canister_id: ic_cdk::api::id(),
            },
        )
        .await
        .unwrap()
        .0
        .settings
        .controllers
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{init, with_state};
    use ic_btc_interface::{Config, Fees, Flag};
    use proptest::prelude::*;

    fn mock_caller(principal: Principal) {
        MOCK_CALLER.with(|cell| *cell.borrow_mut() = Some(principal));
    }

    fn mock_controllers(controllers: Vec<Principal>) {
        MOCK_CONTROLLERS.with(|cell| *cell.borrow_mut() = Some(controllers));
    }

    #[should_panic(expected = "Only controllers can call set_config")]
    #[tokio::test]
    async fn test_set_config_not_watchdog() {
        // Arrange
        let not_watchdog_canister_id = "rwlgt-iiaaa-aaaaa-aaaaa-cai";
        let watchdog_canister_id = "wwc2m-2qaaa-aaaac-qaaaa-cai";
        mock_caller(Principal::from_text(not_watchdog_canister_id).unwrap());
        init(Config {
            watchdog_canister: Some(Principal::from_text(watchdog_canister_id).unwrap()),
            ..Config::default()
        });

        // Act
        set_config(SetConfigRequest {
            api_access: Some(Flag::Disabled),
            ..Default::default()
        })
        .await;
    }

    #[tokio::test]
    async fn test_set_config_watchdog_disables_api_access() {
        // Arrange
        let watchdog_canister_id = "wwc2m-2qaaa-aaaac-qaaaa-cai";
        mock_caller(Principal::from_text(watchdog_canister_id).unwrap());
        init(Config {
            watchdog_canister: Some(Principal::from_text(watchdog_canister_id).unwrap()),
            ..Config::default()
        });
        assert_eq!(with_state(|s| s.api_access), Flag::Enabled);

        // Act
        set_config(SetConfigRequest {
            api_access: Some(Flag::Disabled),
            ..Default::default()
        })
        .await;

        // Assert
        assert_eq!(with_state(|s| s.api_access), Flag::Disabled);
    }

    #[tokio::test]
    async fn test_set_config_watchdog_cant_modify_syncing() {
        // Arrange
        let watchdog_canister_id = "wwc2m-2qaaa-aaaac-qaaaa-cai";
        mock_caller(Principal::from_text(watchdog_canister_id).unwrap());
        init(Config {
            watchdog_canister: Some(Principal::from_text(watchdog_canister_id).unwrap()),
            ..Config::default()
        });
        assert_eq!(with_state(|s| s.syncing_state.syncing), Flag::Enabled);

        // Act
        set_config(SetConfigRequest {
            syncing: Some(Flag::Disabled),
            ..Default::default()
        })
        .await;

        // Assert
        assert_eq!(with_state(|s| s.syncing_state.syncing), Flag::Enabled);
    }

    #[should_panic(expected = "Only controllers can call set_config")]
    #[tokio::test]
    async fn test_set_config_not_controllers() {
        // Arrange
        let not_controller_id = "rwlgt-iiaaa-aaaaa-aaaaa-cai";
        let controller_id = "wwc2m-2qaaa-aaaac-qaaaa-cai";
        mock_caller(Principal::from_text(not_controller_id).unwrap());
        mock_controllers(vec![Principal::from_text(controller_id).unwrap()]);
        init(Config::default());
        assert_eq!(with_state(|s| s.api_access), Flag::Enabled);

        // Act
        set_config(SetConfigRequest {
            api_access: Some(Flag::Disabled),
            ..Default::default()
        })
        .await;
    }

    #[tokio::test]
    async fn test_set_config_controllers() {
        // Arrange
        let controller_id = "wwc2m-2qaaa-aaaac-qaaaa-cai";
        mock_caller(Principal::from_text(controller_id).unwrap());
        mock_controllers(vec![Principal::from_text(controller_id).unwrap()]);
        init(Config::default());
        assert_eq!(with_state(|s| s.api_access), Flag::Enabled);

        // Act
        set_config(SetConfigRequest {
            api_access: Some(Flag::Disabled),
            ..Default::default()
        })
        .await;

        // Assert
        assert_eq!(with_state(|s| s.api_access), Flag::Disabled);
    }

    #[test]
    fn set_stability_threshold() {
        init(Config::default());

        proptest!(|(
            stability_threshold in 0..150u128,
        )| {
            set_config_no_verification(SetConfigRequest {
                stability_threshold: Some(stability_threshold),
                ..Default::default()
            });

            assert_eq!(
                with_state(|s| s.unstable_blocks.stability_threshold()),
                stability_threshold as u32
            );
        });
    }

    #[test]
    fn set_syncing() {
        init(Config::default());

        for flag in &[Flag::Enabled, Flag::Disabled] {
            set_config_no_verification(SetConfigRequest {
                syncing: Some(*flag),
                ..Default::default()
            });

            assert_eq!(
                with_state(|s| s.syncing_state.syncing),
                *flag
            );
        }
    }

    #[test]
    fn set_fees() {
        init(Config::default());

        proptest!(|(
            get_utxos_base in 0..1_000_000_000_000u128,
            get_utxos_maximum in 0..1_000_000_000_000u128,
            get_utxos_cycles_per_ten_instructions in 0..100u128,
            get_balance_maximum in 0..1_000_000_000_000u128,
            get_balance in 0..1_000_000_000_000u128,
            get_current_fee_percentiles in 0..1_000_000_000_000u128,
            get_current_fee_percentiles_maximum in 0..1_000_000_000_000u128,
            send_transaction_base in 0..1_000_000_000_000u128,
            send_transaction_per_byte in 0..1_000_000_000_000u128,
        )| {
            let fees = Fees {
                get_utxos_base,
                get_utxos_maximum,
                get_utxos_cycles_per_ten_instructions,
                get_balance_maximum,
                get_balance,
                get_current_fee_percentiles,
                get_current_fee_percentiles_maximum,
                send_transaction_base,
                send_transaction_per_byte
            };

            set_config_no_verification(SetConfigRequest {
                fees: Some(fees.clone()),
                ..Default::default()
            });

            with_state(|s| assert_eq!(s.fees, fees));
        });
    }

    #[test]
    fn set_api_access() {
        init(Config::default());

        for flag in &[Flag::Enabled, Flag::Disabled] {
            set_config_no_verification(SetConfigRequest {
                api_access: Some(*flag),
                ..Default::default()
            });

            assert_eq!(
                with_state(|s| s.api_access),
                *flag
            );
        }
    }

    #[test]
    fn set_disable_api_if_not_fully_synced() {
        init(Config::default());

        for flag in &[Flag::Enabled, Flag::Disabled] {
            set_config_no_verification(SetConfigRequest {
                disable_api_if_not_fully_synced: Some(*flag),
                ..Default::default()
            });

            assert_eq!(with_state(|s| s.disable_api_if_not_fully_synced), *flag);
        }
    }
}
