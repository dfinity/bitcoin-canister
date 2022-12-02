use crate::SetConfigRequest;
use std::convert::TryInto;

pub async fn set_config(request: SetConfigRequest) {
    verify_caller().await;
    set_config_no_verification(request);
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
    });
}

async fn verify_caller() {
    #[cfg(target_arch = "wasm32")]
    {
        use ic_cdk::api::management_canister::main::CanisterIdRecord;

        let controllers =
            ic_cdk::api::management_canister::main::canister_status(CanisterIdRecord {
                canister_id: ic_cdk::api::id(),
            })
            .await
            .unwrap()
            .0
            .settings
            .controllers;

        if !controllers.contains(&ic_cdk::caller()) {
            panic!("Only controllers can call set_config");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        init,
        types::{Config, Fees, Flag},
        with_state,
    };
    use proptest::prelude::*;

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
}
