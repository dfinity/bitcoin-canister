//! Verifies that the bitcoin canister charges the configured cycle fee even
//! when a request is rejected.
//!
//! Mirrors the dfx-based `charge-cycles-on-reject.sh`, which deployed the
//! canister with a known fee schedule and, for a series of invalid requests,
//! checked that the call was rejected with the expected error *and* that
//! exactly the configured fee was charged.
//!
//! The endpoints accept cycles via `msg_cycles_accept` before producing the
//! error and reject explicitly (rather than trapping), so the accepted cycles
//! are retained. Ingress messages carry no cycles, so calls are routed through
//! the scenario-1 canister's `proxy_call`, which attaches cycles and reports
//! how many the bitcoin canister charged.

use candid::{CandidType, Principal};
use e2e_test_utils::Setup;
use ic_btc_interface::{
    Fees, GetBalanceRequest, GetBlockHeadersRequest, GetUtxosRequest, InitConfig, Network,
    NetworkInRequest, SendTransactionRequest, UtxosFilterInRequest,
};
use scenario_1::{ProxyCallResult, ADDRESS_1};
use serde_bytes::ByteBuf;

/// Cycles attached to every proxied call: generously above every configured
/// `*_maximum` fee, so `verify_has_enough_cycles` always passes and the unused
/// remainder is refunded. The bitcoin canister accepts only the actual fee.
const ATTACHED_CYCLES: u128 = 1_000_000;

/// A flat fee of 1 cycle, configured for every fee field below. With this
/// schedule a rejected `send_transaction` for an N-byte transaction is charged
/// `send_transaction_base + N * send_transaction_per_byte`, and every other
/// rejected request is charged its flat base fee of 1.
fn unit_fees() -> Fees {
    Fees {
        get_utxos_base: 1,
        get_utxos_cycles_per_ten_instructions: 1,
        get_utxos_maximum: 1,
        get_balance: 1,
        get_balance_maximum: 1,
        get_current_fee_percentiles: 1,
        get_current_fee_percentiles_maximum: 1,
        send_transaction_base: 1,
        send_transaction_per_byte: 1,
        get_block_headers_base: 1,
        get_block_headers_cycles_per_ten_instructions: 1,
        get_block_headers_maximum: 1,
    }
}

struct Harness {
    setup: Setup,
    proxy: Principal,
}

impl Harness {
    fn new() -> Self {
        let setup = Setup::new(
            "E2E_SCENARIO_1_WASM_PATH",
            "scenario-1",
            InitConfig {
                stability_threshold: Some(2),
                network: Some(Network::Regtest),
                syncing: Some(ic_btc_interface::Flag::Enabled),
                // The behaviour under test is cycle charging, not syncing. The
                // blocks source serves blocks up to height 5, so gating the API
                // on full sync would be racy; disable it and just sync far
                // enough (height >= 1) for the get_block_headers cases below.
                disable_api_if_not_fully_synced: Some(ic_btc_interface::Flag::Disabled),
                fees: Some(unit_fees()),
                ..Default::default()
            },
        );
        // The block-headers cases need a non-empty chain: `start_height = 1`
        // must be a valid height for the start>end check to fire.
        setup.tick_until_main_chain_height(1, 60);
        let proxy = setup
            .source_id
            .expect("scenario-1 source canister installed");
        Self { setup, proxy }
    }

    /// Routes `request` to `method` on the bitcoin canister through the proxy,
    /// asserting the call is rejected with a message containing `expected_error`
    /// and that exactly `expected_fee` cycles were charged.
    fn check_charging<A: CandidType>(
        &self,
        method: &str,
        request: A,
        expected_error: &str,
        expected_fee: u128,
    ) {
        let arg = candid::encode_one(request).expect("encode request");
        let bytes = self
            .setup
            .pic
            .update_call(
                self.proxy,
                Principal::anonymous(),
                "proxy_call",
                candid::encode_args((self.setup.btc_id, method.to_string(), arg, ATTACHED_CYCLES))
                    .expect("encode proxy_call args"),
            )
            .expect("proxy_call should reply");
        let result: ProxyCallResult = candid::decode_one(&bytes).expect("decode ProxyCallResult");

        let reject = result
            .reject_message
            .unwrap_or_else(|| panic!("{method} unexpectedly succeeded"));
        assert!(
            reject.contains(expected_error),
            "{method}: expected reject containing {expected_error:?}, got {reject:?}"
        );
        assert_eq!(
            result.charged_cycles, expected_fee,
            "{method}: expected to charge {expected_fee} cycles on reject, got {}",
            result.charged_cycles
        );
    }
}

fn balance_req(address: &str, min_confirmations: Option<u32>) -> GetBalanceRequest {
    GetBalanceRequest {
        address: address.to_string(),
        network: NetworkInRequest::Regtest,
        min_confirmations,
    }
}

fn utxos_req(address: &str, filter: Option<UtxosFilterInRequest>) -> GetUtxosRequest {
    GetUtxosRequest {
        address: address.to_string(),
        network: NetworkInRequest::Regtest,
        filter,
    }
}

fn block_headers_req(start_height: u32, end_height: Option<u32>) -> GetBlockHeadersRequest {
    GetBlockHeadersRequest {
        start_height,
        end_height,
        network: NetworkInRequest::Regtest,
    }
}

#[test]
fn charges_configured_fee_on_rejected_requests() {
    let h = Harness::new();

    // bitcoin_send_transaction: malformed 14-byte transaction.
    // Fee = send_transaction_base (1) + 14 * send_transaction_per_byte (1) = 15.
    h.check_charging(
        "bitcoin_send_transaction",
        SendTransactionRequest {
            transaction: b"12341234789789".to_vec(),
            network: NetworkInRequest::Regtest,
        },
        "MalformedTransaction",
        15,
    );

    // bitcoin_get_balance
    h.check_charging(
        "bitcoin_get_balance",
        balance_req("Bad address", None),
        "MalformedAddress",
        1,
    );
    h.check_charging(
        "bitcoin_get_balance",
        balance_req(ADDRESS_1, Some(10)),
        "MinConfirmationsTooLarge",
        1,
    );

    // bitcoin_get_utxos
    h.check_charging(
        "bitcoin_get_utxos",
        utxos_req("Bad address", None),
        "MalformedAddress",
        1,
    );
    h.check_charging(
        "bitcoin_get_utxos",
        utxos_req(ADDRESS_1, Some(UtxosFilterInRequest::MinConfirmations(10))),
        "MinConfirmationsTooLarge",
        1,
    );
    h.check_charging(
        "bitcoin_get_utxos",
        utxos_req(
            ADDRESS_1,
            Some(UtxosFilterInRequest::Page(ByteBuf::from(
                b"12341234789789".to_vec(),
            ))),
        ),
        "MalformedPage",
        1,
    );
    h.check_charging(
        "bitcoin_get_utxos",
        utxos_req(
            ADDRESS_1,
            Some(UtxosFilterInRequest::Page(ByteBuf::from(
                b"123412347897123412347897123412347897123412347897123412347897123412347897"
                    .to_vec(),
            ))),
        ),
        "UnknownTipBlockHash",
        1,
    );

    // bitcoin_get_block_headers
    h.check_charging(
        "bitcoin_get_block_headers",
        block_headers_req(10, None),
        "StartHeightDoesNotExist",
        1,
    );
    h.check_charging(
        "bitcoin_get_block_headers",
        block_headers_req(0, Some(10)),
        "EndHeightDoesNotExist",
        1,
    );
    h.check_charging(
        "bitcoin_get_block_headers",
        block_headers_req(1, Some(0)),
        "StartHeightLargerThanEndHeight",
        1,
    );
}
