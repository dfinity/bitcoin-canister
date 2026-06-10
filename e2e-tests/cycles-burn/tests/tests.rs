//! Verifies that the bitcoin canister burns its cycle balance when started
//! with `burn_cycles = enabled`.
//!
//! Mirrors the original dfx-based `cycles_burn.sh`, which deployed the canister
//! with cycles and `burn_cycles = enabled` and checked that its balance dropped
//! to zero. The burn happens in the heartbeat; under PocketIC, the heartbeat
//! already runs as part of installation, so the balance is drained by the time
//! the canister is up. We tick a few more times for robustness regardless.

use e2e_test_utils::Setup;
use ic_btc_interface::{Flag, InitConfig, Network};

const TICKS: usize = 5;

fn setup_with_burn_cycles(flag: Flag) -> Setup {
    let setup = Setup::new_bitcoin_only(InitConfig {
        network: Some(Network::Regtest),
        burn_cycles: Some(flag),
        ..Default::default()
    });
    for _ in 0..TICKS {
        setup.pic.tick();
    }
    setup
}

#[test]
fn burn_cycles_enabled_drains_the_balance() {
    let enabled = setup_with_burn_cycles(Flag::Enabled);
    assert_eq!(
        enabled.pic.cycle_balance(enabled.btc_id),
        0,
        "burn_cycles = enabled should burn the canister's balance to zero"
    );

    // Sanity check that the flag is what drives the balance to zero: an
    // otherwise identical canister with burn_cycles disabled keeps the cycles
    // it was funded with. This also guards against a false pass if the canister
    // were ever installed unfunded.
    let disabled = setup_with_burn_cycles(Flag::Disabled);
    assert!(
        disabled.pic.cycle_balance(disabled.btc_id) > 0,
        "burn_cycles = disabled should retain the funded balance"
    );
}
