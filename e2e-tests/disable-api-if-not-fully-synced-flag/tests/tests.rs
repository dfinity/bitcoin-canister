//! Verifies that the bitcoin canister honours the `disable_api_if_not_fully_synced`
//! flag.
//!
//! Mirrors the original dfx-based `disable-api-if-not-fully-synced-flag.sh`. The
//! source canister serves 2 full blocks plus 3 "next" block headers, so the
//! canister's main chain reaches height 2 while it knows of headers up to height
//! 5. With `SYNCED_THRESHOLD = 2` that gap (2 + 2 < 5) leaves the canister
//! permanently "not fully synced" — the source never serves the missing full
//! blocks — which is exactly the state the flag guards against:
//!
//! * with the flag `enabled`, every `bitcoin_*` API call traps with
//!   "Canister state is not fully synced.";
//! * with the flag `disabled`, the same calls succeed and return real data.

use disable_api_if_not_fully_synced_flag::ADDRESS;
use e2e_test_utils::{query_raw, update_raw, Setup};
use ic_btc_interface::{
    Flag, GetBalanceRequest, GetBlockHeadersRequest, GetCurrentFeePercentilesRequest,
    GetUtxosRequest, InitConfig, MillisatoshiPerByte, Network, NetworkInRequest,
};

// The source canister advances the main chain to height 2 (see its `NUM_BLOCKS`).
const MAIN_CHAIN_HEIGHT: u32 = 2;
// Generous ceiling for the per-heartbeat block ingestion; height 2 is reached in
// a handful of ticks.
const MAX_TICKS: u32 = 200;

const SOURCE_WASM_ENV: &str = "E2E_DISABLE_API_WASM_PATH";
const SOURCE_NAME: &str = "disable-api-if-not-fully-synced-flag";

fn setup(flag: Flag) -> Setup {
    let setup = Setup::new(
        SOURCE_WASM_ENV,
        SOURCE_NAME,
        InitConfig {
            stability_threshold: Some(1),
            network: Some(Network::Regtest),
            disable_api_if_not_fully_synced: Some(flag),
            ..Default::default()
        },
    );
    setup.tick_until_main_chain_height(MAIN_CHAIN_HEIGHT, MAX_TICKS);
    setup
}

fn balance_req() -> GetBalanceRequest {
    GetBalanceRequest {
        address: ADDRESS.to_string(),
        network: NetworkInRequest::Regtest,
        min_confirmations: None,
    }
}

fn utxos_req() -> GetUtxosRequest {
    GetUtxosRequest {
        address: ADDRESS.to_string(),
        network: NetworkInRequest::Regtest,
        filter: None,
    }
}

fn block_headers_req() -> GetBlockHeadersRequest {
    GetBlockHeadersRequest {
        start_height: 0,
        end_height: None,
        network: NetworkInRequest::Regtest,
    }
}

fn fee_percentiles_req() -> GetCurrentFeePercentilesRequest {
    GetCurrentFeePercentilesRequest {
        network: NetworkInRequest::Regtest,
    }
}

#[test]
fn api_rejected_when_enabled_and_not_synced() {
    let setup = setup(Flag::Enabled);

    // Every API call — update and query alike — must trap while not fully synced.
    for (method, result) in [
        (
            "bitcoin_get_balance",
            update_raw(
                &setup.pic,
                setup.btc_id,
                "bitcoin_get_balance",
                balance_req(),
            ),
        ),
        (
            "bitcoin_get_balance_query",
            query_raw(
                &setup.pic,
                setup.btc_id,
                "bitcoin_get_balance_query",
                balance_req(),
            ),
        ),
        (
            "bitcoin_get_utxos",
            update_raw(&setup.pic, setup.btc_id, "bitcoin_get_utxos", utxos_req()),
        ),
        (
            "bitcoin_get_utxos_query",
            query_raw(
                &setup.pic,
                setup.btc_id,
                "bitcoin_get_utxos_query",
                utxos_req(),
            ),
        ),
        (
            "bitcoin_get_block_headers",
            update_raw(
                &setup.pic,
                setup.btc_id,
                "bitcoin_get_block_headers",
                block_headers_req(),
            ),
        ),
        (
            "bitcoin_get_current_fee_percentiles",
            update_raw(
                &setup.pic,
                setup.btc_id,
                "bitcoin_get_current_fee_percentiles",
                fee_percentiles_req(),
            ),
        ),
    ] {
        let err = result.expect_err(&format!(
            "expected {method} to be rejected while not fully synced"
        ));
        assert!(
            err.reject_message
                .contains("Canister state is not fully synced."),
            "{method}: unexpected reject message: {}",
            err.reject_message
        );
    }
}

#[test]
fn api_served_when_disabled_even_if_not_synced() {
    let setup = setup(Flag::Disabled);

    // ADDRESS receives 1 satoshi in each of the 2 generated blocks.
    assert_eq!(
        setup.bitcoin_get_balance(balance_req()),
        2,
        "balance should reflect the 2 satoshis received across the 2 blocks"
    );
    assert_eq!(
        setup.bitcoin_get_utxos(utxos_req()).utxos.len(),
        2,
        "ADDRESS should have one UTXO per generated block"
    );

    let fees: Vec<MillisatoshiPerByte> = e2e_test_utils::update(
        &setup.pic,
        setup.btc_id,
        "bitcoin_get_current_fee_percentiles",
        fee_percentiles_req(),
    );
    assert!(
        fees.is_empty(),
        "fee percentiles should be empty for the regtest chain, got {fees:?}"
    );

    let headers = setup.bitcoin_get_block_headers(block_headers_req());
    assert_eq!(
        headers.tip_height, MAIN_CHAIN_HEIGHT,
        "block headers tip should match the main chain height"
    );
}
