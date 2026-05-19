use e2e_test_utils::{
    bitcoin_get_balance, bitcoin_get_utxos, get_blockchain_info, install_bitcoin_canister,
    install_canister_on_subnet, load_wasm, pocket_ic_with_bitcoin_subnet,
    tick_until_main_chain_height,
};
use ic_btc_interface::{
    GetBalanceRequest, GetUtxosRequest, InitConfig, Network, NetworkInRequest,
};
use scenario_2::ADDRESS;

#[test]
fn scenario_2() {
    let source_wasm = load_wasm("E2E_SCENARIO_2_WASM_PATH", "scenario-2");
    let btc_wasm = load_wasm("IC_BTC_CANISTER_WASM_PATH", "ic-btc-canister");
    let (pic, bitcoin_subnet) = pocket_ic_with_bitcoin_subnet();
    let source_id = install_canister_on_subnet(&pic, bitcoin_subnet, source_wasm, vec![]);
    let btc_id = install_bitcoin_canister(
        &pic,
        bitcoin_subnet,
        InitConfig {
            stability_threshold: Some(1),
            network: Some(Network::Regtest),
            blocks_source: Some(source_id),
            ..Default::default()
        },
        btc_wasm,
    );

    // The scenario-2 source canister populates 4 blocks, each with 10_000 transactions
    // sending 1 satoshi to ADDRESS. It serves one block per get_successors call, so the
    // bitcoin canister needs several heartbeats to ingest them all; 500 ticks is a
    // generous ceiling.
    tick_until_main_chain_height(&pic, btc_id, 4, 500);

    assert_eq!(get_blockchain_info(&pic, btc_id).height, 4);

    // 4 blocks * 10_000 transactions * 1 satoshi = 40_000.
    assert_eq!(
        bitcoin_get_balance(
            &pic,
            btc_id,
            GetBalanceRequest {
                address: ADDRESS.to_string(),
                network: NetworkInRequest::Regtest,
                min_confirmations: None,
            }
        ),
        40_000
    );

    // ADDRESS has 40_000 UTXOs (one per transaction). Responses are capped at 1000.
    let utxos_resp = bitcoin_get_utxos(
        &pic,
        btc_id,
        GetUtxosRequest {
            address: ADDRESS.to_string(),
            network: NetworkInRequest::Regtest,
            filter: None,
        },
    );
    assert_eq!(utxos_resp.utxos.len(), 1000);
}
