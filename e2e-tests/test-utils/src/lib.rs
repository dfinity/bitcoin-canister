//! Shared helpers for the bitcoin-canister PocketIC-based e2e tests.
//!
//! These helpers are the infrastructure pieces extracted from scenario-1's
//! integration test so the other e2e tests don't have to reinvent them.

use candid::{CandidType, Principal};
use cargo_metadata::MetadataCommand;
use escargot::CargoBuild;
use ic_btc_canister::types::{HttpRequest, HttpResponse};
use ic_btc_interface::{
    BlockchainInfo, CanisterArg, GetBalanceRequest, GetBlockHeadersRequest,
    GetBlockHeadersResponse, GetUtxosRequest, GetUtxosResponse, InitConfig, SendTransactionRequest,
};
use pocket_ic::{PocketIc, PocketIcBuilder, RejectResponse};
use serde::de::DeserializeOwned;
use serde_bytes::ByteBuf;
use std::path::PathBuf;

// ---------- WASM loading ----------

/// Loads a canister WASM by reading the path in `env_var`, or — when the env
/// var is unset — by invoking `cargo build` programmatically via `escargot`
/// to produce one for local development.
///
/// On CI the env var must be set; if it isn't, we panic rather than silently
/// shelling out to cargo inside the test process, which would mask a CI
/// misconfiguration as a slow test run.
pub fn load_wasm(env_var: &str, canister_name: &str) -> Vec<u8> {
    if let Ok(path) = std::env::var(env_var) {
        return std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read WASM from {path}: {e}"));
    }
    if std::env::var_os("CI").is_some() {
        panic!(
            "Running on CI and expected env var {env_var} to point at a pre-built \
             {canister_name} WASM. Wire it in the workflow before invoking cargo test."
        );
    }

    // Local-dev fallback: build via escargot into a dedicated target dir so we
    // don't invalidate the native (test-binary) build cache on every iteration.
    let cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("Cargo.toml");
    let target_directory = MetadataCommand::new()
        .manifest_path(&cargo_toml)
        .no_deps()
        .exec()
        .unwrap_or_else(|e| panic!("cargo metadata failed for {}: {e}", cargo_toml.display()))
        .target_directory;
    let wasm_target_dir = PathBuf::from(target_directory.as_str()).join("wasm-cargo-bin");

    let artifact = CargoBuild::new()
        .target("wasm32-unknown-unknown")
        .bin(canister_name)
        .arg("--release")
        .arg("--locked")
        .manifest_path(&cargo_toml)
        .target_dir(&wasm_target_dir)
        .run()
        .unwrap_or_else(|e| panic!("cargo build for {canister_name} failed: {e}"));
    std::fs::read(artifact.path())
        .unwrap_or_else(|e| panic!("failed to read WASM from {}: {e}", artifact.path().display()))
}

// ---------- PocketIC setup ----------

/// Builds a PocketIC instance with a single bitcoin subnet and returns the
/// instance together with the bitcoin subnet id. Co-locating the bitcoin
/// canister and any test fixtures on the bitcoin subnet matches production
/// (the adapter is a node-level service co-located with the bitcoin canister).
pub fn pocket_ic_with_bitcoin_subnet() -> (PocketIc, Principal) {
    let pic = PocketIcBuilder::new().with_bitcoin_subnet().build();
    let bitcoin_subnet = pic
        .topology()
        .get_bitcoin()
        .expect("bitcoin subnet not present");
    (pic, bitcoin_subnet)
}

/// Creates and installs a canister on the given subnet, with 10 trillion
/// cycles pre-funded. `encoded_arg` is the candid-encoded init argument.
pub fn install_canister_on_subnet(
    pic: &PocketIc,
    subnet: Principal,
    wasm: Vec<u8>,
    encoded_arg: Vec<u8>,
) -> Principal {
    let canister_id = pic.create_canister_on_subnet(None, None, subnet);
    pic.add_cycles(canister_id, 10_000_000_000_000);
    pic.install_canister(canister_id, wasm, encoded_arg, None);
    canister_id
}

/// Convenience wrapper that installs the bitcoin canister with a typed
/// `InitConfig` (avoids the per-test boilerplate of wrapping in `CanisterArg::Init`
/// and candid-encoding).
pub fn install_bitcoin_canister(
    pic: &PocketIc,
    subnet: Principal,
    init: InitConfig,
    wasm: Vec<u8>,
) -> Principal {
    let arg = candid::encode_one(CanisterArg::Init(init))
        .expect("failed to encode bitcoin canister InitConfig");
    install_canister_on_subnet(pic, subnet, wasm, arg)
}

// ---------- Generic candid call helpers ----------

/// Candid-encoded query call; panics with a precise message if the call or
/// the response decoding fails.
pub fn query<R: CandidType + DeserializeOwned>(
    pic: &PocketIc,
    canister_id: Principal,
    method: &str,
    arg: impl CandidType,
) -> R {
    let bytes = pic
        .query_call(
            canister_id,
            Principal::anonymous(),
            method,
            candid::encode_one(arg).unwrap(),
        )
        .unwrap_or_else(|e| panic!("{method} query failed: {e:?}"));
    candid::decode_one(&bytes).unwrap_or_else(|e| panic!("decode {method} response: {e}"))
}

/// Candid-encoded update call; panics with a precise message if the call or
/// the response decoding fails.
pub fn update<R: CandidType + DeserializeOwned>(
    pic: &PocketIc,
    canister_id: Principal,
    method: &str,
    arg: impl CandidType,
) -> R {
    let bytes = pic
        .update_call(
            canister_id,
            Principal::anonymous(),
            method,
            candid::encode_one(arg).unwrap(),
        )
        .unwrap_or_else(|e| panic!("{method} update call failed: {e:?}"));
    candid::decode_one(&bytes).unwrap_or_else(|e| panic!("decode {method} response: {e}"))
}

/// Raw update call; returns the canister's response bytes or the reject so
/// tests can assert on rejection codes and messages.
pub fn update_raw(
    pic: &PocketIc,
    canister_id: Principal,
    method: &str,
    arg: impl CandidType,
) -> Result<Vec<u8>, RejectResponse> {
    pic.update_call(
        canister_id,
        Principal::anonymous(),
        method,
        candid::encode_one(arg).unwrap(),
    )
}

// ---------- Bitcoin canister introspection ----------

/// Reads the bitcoin canister's `get_blockchain_info` query endpoint.
pub fn get_blockchain_info(pic: &PocketIc, btc_id: Principal) -> BlockchainInfo {
    let bytes = pic
        .query_call(
            btc_id,
            Principal::anonymous(),
            "get_blockchain_info",
            candid::encode_args(()).unwrap(),
        )
        .expect("get_blockchain_info query failed");
    candid::decode_one(&bytes).expect("failed to decode BlockchainInfo")
}

/// Reads `stable_height` from the bitcoin canister's `/metrics` endpoint.
///
/// `stable_height` is unconditionally encoded as a gauge from the very first
/// request (see `canister/src/api/metrics.rs`), so every step here must
/// succeed; we panic with a precise message rather than mapping failures to
/// `None`, which would mask real bugs behind a timeout.
pub fn get_stable_height(pic: &PocketIc, btc_id: Principal) -> u32 {
    let request = HttpRequest {
        method: "GET".to_string(),
        url: "/metrics".to_string(),
        headers: vec![],
        body: ByteBuf::new(),
    };
    let bytes = pic
        .query_call(
            btc_id,
            Principal::anonymous(),
            "http_request",
            candid::encode_one(request).unwrap(),
        )
        .expect("http_request /metrics query failed");
    let response: HttpResponse =
        candid::decode_one(&bytes).expect("failed to decode /metrics response");
    assert_eq!(
        response.status_code,
        200,
        "metrics endpoint returned {}: {}",
        response.status_code,
        String::from_utf8_lossy(&response.body)
    );
    let body =
        String::from_utf8(response.body.into_vec()).expect("/metrics body is not valid UTF-8");
    // The metric is encoded as f64 but always a whole number; parse as f64 first
    // so this survives any encoder change that emits "3.0" instead of "3".
    // Accept both unlabeled ("stable_height N") and labeled ("stable_height{...} N")
    // forms so a future label addition doesn't silently break the match.
    let line = body
        .lines()
        .find(|line| line.starts_with("stable_height ") || line.starts_with("stable_height{"))
        .expect("stable_height metric not found in /metrics output");
    let value = line
        .split_whitespace()
        .nth(1)
        .expect("stable_height line has no value field");
    value
        .parse::<f64>()
        .unwrap_or_else(|e| panic!("failed to parse stable_height value {value:?}: {e}")) as u32
}

/// Ticks the PocketIC instance until `get_blockchain_info().height >= target`,
/// up to `max_ticks` ticks. Panics with the last observed height on timeout.
pub fn tick_until_main_chain_height(
    pic: &PocketIc,
    btc_id: Principal,
    target: u32,
    max_ticks: u32,
) {
    for _ in 0..max_ticks {
        pic.tick();
        if get_blockchain_info(pic, btc_id).height >= target {
            return;
        }
    }
    panic!("timed out after {max_ticks} ticks waiting for main chain height {target}");
}

/// Ticks the PocketIC instance until `stable_height >= target`, up to
/// `max_ticks` ticks. Panics with the last observed height on timeout.
pub fn tick_until_stable_height(pic: &PocketIc, btc_id: Principal, target: u32, max_ticks: u32) {
    for _ in 0..max_ticks {
        pic.tick();
        if get_stable_height(pic, btc_id) >= target {
            return;
        }
    }
    panic!("timed out after {max_ticks} ticks waiting for stable height {target}");
}

// ---------- Typed bitcoin canister method wrappers ----------

pub fn bitcoin_get_balance(pic: &PocketIc, btc_id: Principal, req: GetBalanceRequest) -> u64 {
    update(pic, btc_id, "bitcoin_get_balance", req)
}

pub fn bitcoin_get_balance_query(pic: &PocketIc, btc_id: Principal, req: GetBalanceRequest) -> u64 {
    query(pic, btc_id, "bitcoin_get_balance_query", req)
}

pub fn bitcoin_get_utxos(
    pic: &PocketIc,
    btc_id: Principal,
    req: GetUtxosRequest,
) -> GetUtxosResponse {
    update(pic, btc_id, "bitcoin_get_utxos", req)
}

pub fn bitcoin_get_utxos_query(
    pic: &PocketIc,
    btc_id: Principal,
    req: GetUtxosRequest,
) -> GetUtxosResponse {
    query(pic, btc_id, "bitcoin_get_utxos_query", req)
}

pub fn bitcoin_get_block_headers(
    pic: &PocketIc,
    btc_id: Principal,
    req: GetBlockHeadersRequest,
) -> GetBlockHeadersResponse {
    update(pic, btc_id, "bitcoin_get_block_headers", req)
}

pub fn bitcoin_send_transaction(pic: &PocketIc, btc_id: Principal, req: SendTransactionRequest) {
    update(pic, btc_id, "bitcoin_send_transaction", req)
}
