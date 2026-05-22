use e2e_test_utils::Setup;
use ic_btc_interface::{InitConfig, Network, SetConfigRequest};

#[test]
fn set_config_updates_stability_threshold() {
    let setup = Setup::new_bitcoin_only(InitConfig {
        stability_threshold: Some(0),
        network: Some(Network::Regtest),
        ..Default::default()
    });

    assert_eq!(setup.get_config().stability_threshold, 0);

    setup.set_config(SetConfigRequest {
        stability_threshold: Some(17),
        ..Default::default()
    });

    assert_eq!(setup.get_config().stability_threshold, 17);
}
