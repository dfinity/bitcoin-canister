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
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }

    #[cfg(target_arch = "wasm32")]
    {
        crate::with_state(|s| Some(ic_cdk::caller()) == s.watchdog_canister)
    }
}

fn set_api_access(request: SetConfigRequest) {
    crate::with_state_mut(|s| {
        if let Some(api_access) = request.api_access {
            s.api_access = api_access;
        }
    });
}

pub(crate) fn set_config_no_verification(request: SetConfigRequest) {
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

        if let Some(watchdog_canister) = request.watchdog_canister {
            s.watchdog_canister = watchdog_canister;
        }

        if let Some(lazily_evaluate_fee_percentiles) = request.lazily_evaluate_fee_percentiles {
            s.lazily_evaluate_fee_percentiles = lazily_evaluate_fee_percentiles;
        }
    });
}

async fn verify_caller() {
    #[cfg(target_arch = "wasm32")]
    {
        use ic_cdk::api::management_canister::main::CanisterIdRecord;

        let caller = ic_cdk::caller();
        let controllers =
            ic_cdk::api::management_canister::main::canister_status(CanisterIdRecord {
                canister_id: ic_cdk::api::id(),
            })
            .await
            .unwrap()
            .0
            .settings
            .controllers;

        if !controllers.contains(&caller) {
            panic!("Only controllers can call set_config");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{init, with_state};
    use candid::Principal;
    use ic_btc_interface::{Fees, Flag, InitConfig};
    use proptest::prelude::*;

    #[test]
    fn test_set_api_access_updates_state() {
        // Arrange
        init(InitConfig::default());
        assert_eq!(with_state(|s| s.api_access), Flag::Enabled);

        // Act
        set_api_access(SetConfigRequest {
            api_access: Some(Flag::Disabled),
            ..Default::default()
        });

        // Assert
        assert_eq!(with_state(|s| s.api_access), Flag::Disabled);
    }

    #[test]
    fn test_set_api_access_does_not_update_state() {
        // Arrange
        init(InitConfig::default());
        assert_eq!(with_state(|s| s.syncing_state.syncing), Flag::Enabled);

        // Act
        set_api_access(SetConfigRequest {
            syncing: Some(Flag::Disabled),
            ..Default::default()
        });

        // Assert
        assert_eq!(with_state(|s| s.syncing_state.syncing), Flag::Enabled);
    }

    #[test]
    fn test_set_stability_threshold() {
        init(InitConfig::default());

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
    fn test_set_syncing() {
        init(InitConfig::default());

        for flag in &[Flag::Enabled, Flag::Disabled] {
            set_config_no_verification(SetConfigRequest {
                syncing: Some(*flag),
                ..Default::default()
            });

            assert_eq!(with_state(|s| s.syncing_state.syncing), *flag);
        }
    }

    #[test]
    fn test_set_fees() {
        init(InitConfig::default());

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
            get_block_headers_base in 0..1_000_000_000_000u128,
            get_block_headers_cycles_per_ten_instructions in 0..1_000_000_000_000u128,
            get_block_headers_maximum in 0..1_000_000_000_000u128,
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
                send_transaction_per_byte,
                get_block_headers_base,
                get_block_headers_cycles_per_ten_instructions,
                get_block_headers_maximum
            };

            set_config_no_verification(SetConfigRequest {
                fees: Some(fees.clone()),
                ..Default::default()
            });

            with_state(|s| assert_eq!(s.fees, fees));
        });
    }

    #[test]
    fn test_set_config_no_verification_for_setting_api_access() {
        init(InitConfig::default());

        for flag in &[Flag::Enabled, Flag::Disabled] {
            set_config_no_verification(SetConfigRequest {
                api_access: Some(*flag),
                ..Default::default()
            });

            assert_eq!(with_state(|s| s.api_access), *flag);
        }
    }

    #[test]
    fn test_set_disable_api_if_not_fully_synced() {
        init(InitConfig::default());

        for flag in &[Flag::Enabled, Flag::Disabled] {
            set_config_no_verification(SetConfigRequest {
                disable_api_if_not_fully_synced: Some(*flag),
                ..Default::default()
            });

            assert_eq!(with_state(|s| s.disable_api_if_not_fully_synced), *flag);
        }
    }

    #[test]
    fn test_set_watchdog_canister() {
        init(InitConfig::default());

        for watchdog_canister in [
            None,
            Some(Principal::anonymous()),
            Some(Principal::management_canister()),
            Some(Principal::from_text("g4xu7-jiaaa-aaaan-aaaaq-cai").unwrap()),
        ] {
            set_config_no_verification(SetConfigRequest {
                watchdog_canister: Some(watchdog_canister),
                ..Default::default()
            });

            assert_eq!(with_state(|s| s.watchdog_canister), watchdog_canister);
        }
    }

    #[test]
    fn test_set_lazily_evaluate_fee_percentiles() {
        init(InitConfig::default());

        for flag in &[Flag::Enabled, Flag::Disabled] {
            set_config_no_verification(SetConfigRequest {
                lazily_evaluate_fee_percentiles: Some(*flag),
                ..Default::default()
            });

            assert_eq!(with_state(|s| s.lazily_evaluate_fee_percentiles), *flag);
        }
    }
}
