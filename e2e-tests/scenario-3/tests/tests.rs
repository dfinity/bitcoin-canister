use e2e_test_utils::{query, update_raw, Setup};
use ic_btc_interface::{InitConfig, Network, NetworkInRequest, SendTransactionRequest};
use serde_bytes::ByteBuf;

#[test]
fn scenario_3() {
    let setup = Setup::new(
        "E2E_SCENARIO_3_WASM_PATH",
        "scenario-3",
        InitConfig {
            stability_threshold: Some(2),
            network: Some(Network::Regtest),
            ..Default::default()
        },
    );

    // Send a valid (empty segwit-encoded) transaction. bitcoin_send_transaction awaits
    // the inter-canister call to bitcoin_send_transaction_internal on the source, so
    // by the time it returns LAST_TRANSACTION on the source has been updated.
    let valid_tx: Vec<u8> = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    setup.bitcoin_send_transaction(SendTransactionRequest {
        network: NetworkInRequest::Regtest,
        transaction: valid_tx.clone(),
    });

    let last_tx: ByteBuf = query(&setup.pic, setup.source_id, "get_last_transaction", ());
    assert_eq!(last_tx.as_slice(), valid_tx.as_slice());

    // Send an invalid transaction; the canister must reject with MalformedTransaction.
    let invalid_tx = b"12341234789789".to_vec();
    let reject = update_raw(
        &setup.pic,
        setup.btc_id,
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
