use e2e_test_utils::Setup;
use ic_btc_interface::{InitConfig, Network, SetConfigRequest};

#[test]
fn set_config_updates_stability_threshold() {
    let setup = Setup::new_bitcoin_only(InitConfig {
        stability_threshold: Some(0),
        network: Some(Network::Regtest),
        ..Default::default()
    });

    // Verify the init-time stability threshold is visible via get_config.
    assert_eq!(setup.get_config().stability_threshold, 0);

    // Update the stability threshold via set_config.
    setup.set_config(SetConfigRequest {
        stability_threshold: Some(17),
        ..Default::default()
    });

    // Verify the new value is visible via get_config.
    assert_eq!(setup.get_config().stability_threshold, 17);
}
