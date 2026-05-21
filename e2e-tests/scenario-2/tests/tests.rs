use e2e_test_utils::Setup;
use ic_btc_interface::{GetBalanceRequest, GetUtxosRequest, InitConfig, Network, NetworkInRequest};
use scenario_2::ADDRESS;

#[test]
fn scenario_2() {
    let setup = Setup::new(
        "E2E_SCENARIO_2_WASM_PATH",
        "scenario-2",
        InitConfig {
            stability_threshold: Some(1),
            network: Some(Network::Regtest),
            ..Default::default()
        },
    );

    // The scenario-2 source canister populates 4 blocks, each with 10_000 transactions
    // sending 1 satoshi to ADDRESS. It serves one block per get_successors call, so the
    // bitcoin canister needs several heartbeats to ingest them all; 500 ticks is a
    // generous ceiling.
    setup.tick_until_main_chain_height(4, 500);

    assert_eq!(setup.get_blockchain_info().height, 4);

    // 4 blocks * 10_000 transactions * 1 satoshi = 40_000.
    assert_eq!(
        setup.bitcoin_get_balance(GetBalanceRequest {
            address: ADDRESS.to_string(),
            network: NetworkInRequest::Regtest,
            min_confirmations: None,
        }),
        40_000
    );

    // ADDRESS has 40_000 UTXOs (one per transaction). Responses are capped at 1000.
    let utxos_resp = setup.bitcoin_get_utxos(GetUtxosRequest {
        address: ADDRESS.to_string(),
        network: NetworkInRequest::Regtest,
        filter: None,
    });
    assert_eq!(utxos_resp.utxos.len(), 1000);
}
