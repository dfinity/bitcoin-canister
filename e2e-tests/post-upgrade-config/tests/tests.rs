use e2e_test_utils::Setup;
use ic_btc_interface::{Fees, InitConfig, Network, SetConfigRequest};

#[test]
fn post_upgrade_applies_set_config_request() {
    let setup = Setup::new_bitcoin_only(InitConfig {
        stability_threshold: Some(0),
        network: Some(Network::Regtest),
        ..Default::default()
    });

    assert_eq!(setup.get_config().stability_threshold, 0);

    setup.upgrade_bitcoin_canister(Some(SetConfigRequest {
        fees: Some(Fees {
            get_current_fee_percentiles: 123,
            ..Default::default()
        }),
        ..Default::default()
    }));

    assert_eq!(setup.get_config().fees.get_current_fee_percentiles, 123);
}
