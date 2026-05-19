use e2e_test_utils::{
    bitcoin_send_transaction, install_bitcoin_canister, install_canister_on_subnet, load_wasm,
    pocket_ic_with_bitcoin_subnet, query, update_raw,
};
use ic_btc_interface::{InitConfig, Network, NetworkInRequest, SendTransactionRequest};
use serde_bytes::ByteBuf;

#[test]
fn scenario_3() {
    let source_wasm = load_wasm("E2E_SCENARIO_3_WASM_PATH", "scenario-3");
    let btc_wasm = load_wasm("IC_BTC_CANISTER_WASM_PATH", "ic-btc-canister");
    let (pic, bitcoin_subnet) = pocket_ic_with_bitcoin_subnet();
    let source_id = install_canister_on_subnet(&pic, bitcoin_subnet, source_wasm, vec![]);
    let btc_id = install_bitcoin_canister(
        &pic,
        bitcoin_subnet,
        InitConfig {
            stability_threshold: Some(2),
            network: Some(Network::Regtest),
            blocks_source: Some(source_id),
            ..Default::default()
        },
        btc_wasm,
    );

    // Send a valid (empty segwit-encoded) transaction. bitcoin_send_transaction awaits
    // the inter-canister call to bitcoin_send_transaction_internal on the source, so
    // by the time it returns LAST_TRANSACTION on the source has been updated.
    let valid_tx: Vec<u8> = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    bitcoin_send_transaction(
        &pic,
        btc_id,
        SendTransactionRequest {
            network: NetworkInRequest::Regtest,
            transaction: valid_tx.clone(),
        },
    );

    let last_tx: ByteBuf = query(&pic, source_id, "get_last_transaction", ());
    assert_eq!(last_tx.as_slice(), valid_tx.as_slice());

    // Send an invalid transaction; the canister must reject with MalformedTransaction.
    let invalid_tx = b"12341234789789".to_vec();
    let reject = update_raw(
        &pic,
        btc_id,
        "bitcoin_send_transaction",
        SendTransactionRequest {
            network: NetworkInRequest::Regtest,
            transaction: invalid_tx,
        },
    )
    .expect_err("expected bitcoin_send_transaction with invalid bytes to be rejected");
    assert!(
        reject.reject_message.contains("MalformedTransaction"),
        "unexpected reject message: {}",
        reject.reject_message
    );
}
