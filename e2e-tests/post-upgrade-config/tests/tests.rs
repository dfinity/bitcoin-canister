use e2e_test_utils::{
    get_config, install_bitcoin_canister, load_wasm, pocket_ic_with_bitcoin_subnet,
    upgrade_bitcoin_canister,
};
use ic_btc_interface::{Fees, InitConfig, Network, SetConfigRequest};

#[test]
fn post_upgrade_applies_set_config_request() {
    let btc_wasm = load_wasm("IC_BTC_CANISTER_WASM_PATH", "ic-btc-canister");
    let (pic, bitcoin_subnet) = pocket_ic_with_bitcoin_subnet();
    let btc_id = install_bitcoin_canister(
        &pic,
        bitcoin_subnet,
        InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Regtest),
            ..Default::default()
        },
        btc_wasm.clone(),
    );

    // After install, stability_threshold should reflect what InitConfig set.
    assert_eq!(get_config(&pic, btc_id).stability_threshold, 0);

    // Upgrade with new fees: only `get_current_fee_percentiles` is non-zero,
    // matching the original shell test. `Fees::default()` zero-initialises every
    // field, so we override only the one we care about.
    upgrade_bitcoin_canister(
        &pic,
        btc_id,
        btc_wasm,
        Some(SetConfigRequest {
            fees: Some(Fees {
                get_current_fee_percentiles: 123,
                ..Default::default()
            }),
            ..Default::default()
        }),
    );

    // Verify post_upgrade applied the SetConfigRequest.
    assert_eq!(
        get_config(&pic, btc_id).fees.get_current_fee_percentiles,
        123
    );
}
