//! Shared helpers for the bitcoin-canister PocketIC-based e2e tests.
//!
//! These helpers are the infrastructure pieces extracted from scenario-1's
//! integration test so the other e2e tests don't have to reinvent them.

use candid::{CandidType, Principal};
use cargo_metadata::MetadataCommand;
use escargot::CargoBuild;
use ic_btc_canister::types::{HttpRequest, HttpResponse};
use ic_btc_interface::{
    BlockchainInfo, CanisterArg, Config, GetBalanceRequest, GetBlockHeadersRequest,
    GetBlockHeadersResponse, GetUtxosRequest, GetUtxosResponse, InitConfig, SendTransactionRequest,
    SetConfigRequest,
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
    std::fs::read(artifact.path()).unwrap_or_else(|e| {
        panic!(
            "failed to read WASM from {}: {e}",
            artifact.path().display()
        )
    })
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

/// Upgrades the bitcoin canister, wrapping the optional `SetConfigRequest` in
/// `CanisterArg::Upgrade` and candid-encoding it. Panics on reject.
pub fn upgrade_bitcoin_canister(
    pic: &PocketIc,
    btc_id: Principal,
    wasm: Vec<u8>,
    set_config: Option<SetConfigRequest>,
) {
    let arg = candid::encode_one(CanisterArg::Upgrade(set_config))
        .expect("failed to encode bitcoin canister CanisterArg::Upgrade");
    pic.upgrade_canister(btc_id, wasm, arg, None)
        .unwrap_or_else(|e| panic!("bitcoin canister upgrade failed: {e:?}"));
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

pub fn get_config(pic: &PocketIc, btc_id: Principal) -> Config {
    query(pic, btc_id, "get_config", ())
}

pub fn set_config(pic: &PocketIc, btc_id: Principal, req: SetConfigRequest) {
    update(pic, btc_id, "set_config", req)
}

// ---------- Setup ----------

pub struct Setup {
    pub pic: PocketIc,
    pub btc_id: Principal,
    pub source_id: Option<Principal>,
}

impl Setup {
    /// Brings up a fresh PocketIC instance with a bitcoin subnet, installs
    /// the source canister, then installs the bitcoin canister with
    /// `init.blocks_source` wired to the source canister id.
    ///
    /// Panics if `init.blocks_source` is `Some(_)`: the source canister id
    /// is created inside this constructor, so a caller-provided value can
    /// only be a mistake.
    pub fn new(source_wasm_env: &str, source_name: &str, init: InitConfig) -> Self {
        assert!(
            init.blocks_source.is_none(),
            "Setup::new wires blocks_source itself; caller must leave it as None",
        );
        let source_wasm = load_wasm(source_wasm_env, source_name);
        let btc_wasm = load_wasm("IC_BTC_CANISTER_WASM_PATH", "ic-btc-canister");
        let (pic, bitcoin_subnet) = pocket_ic_with_bitcoin_subnet();
        let source_id = install_canister_on_subnet(&pic, bitcoin_subnet, source_wasm, vec![]);
        let init = InitConfig {
            blocks_source: Some(source_id),
            ..init
        };
        let btc_id = install_bitcoin_canister(&pic, bitcoin_subnet, init, btc_wasm);
        Self {
            pic,
            btc_id,
            source_id: Some(source_id),
        }
    }

    /// Brings up a fresh PocketIC instance with a bitcoin subnet and
    /// installs only the bitcoin canister — no source canister. Use this
    /// for tests that don't ingest blocks (e.g. config mutations,
    /// metadata checks).
    ///
    /// Panics if `init.blocks_source` is `Some(_)`: no source canister is
    /// installed here, so any value supplied would point at a non-existent
    /// principal and silently break block ingestion.
    pub fn new_bitcoin_only(init: InitConfig) -> Self {
        assert!(
            init.blocks_source.is_none(),
            "Setup::new_bitcoin_only installs no source canister; \
             caller must leave blocks_source as None",
        );
        let btc_wasm = load_wasm("IC_BTC_CANISTER_WASM_PATH", "ic-btc-canister");
        let (pic, bitcoin_subnet) = pocket_ic_with_bitcoin_subnet();
        let btc_id = install_bitcoin_canister(&pic, bitcoin_subnet, init, btc_wasm);
        Self {
            pic,
            btc_id,
            source_id: None,
        }
    }

    pub fn get_blockchain_info(&self) -> BlockchainInfo {
        get_blockchain_info(&self.pic, self.btc_id)
    }

    pub fn get_stable_height(&self) -> u32 {
        get_stable_height(&self.pic, self.btc_id)
    }

    pub fn tick_until_main_chain_height(&self, target: u32, max_ticks: u32) {
        tick_until_main_chain_height(&self.pic, self.btc_id, target, max_ticks)
    }

    pub fn tick_until_stable_height(&self, target: u32, max_ticks: u32) {
        tick_until_stable_height(&self.pic, self.btc_id, target, max_ticks)
    }

    pub fn bitcoin_get_balance(&self, req: GetBalanceRequest) -> u64 {
        bitcoin_get_balance(&self.pic, self.btc_id, req)
    }

    pub fn bitcoin_get_balance_query(&self, req: GetBalanceRequest) -> u64 {
        bitcoin_get_balance_query(&self.pic, self.btc_id, req)
    }

    pub fn bitcoin_get_utxos(&self, req: GetUtxosRequest) -> GetUtxosResponse {
        bitcoin_get_utxos(&self.pic, self.btc_id, req)
    }

    pub fn bitcoin_get_utxos_query(&self, req: GetUtxosRequest) -> GetUtxosResponse {
        bitcoin_get_utxos_query(&self.pic, self.btc_id, req)
    }

    pub fn bitcoin_get_block_headers(
        &self,
        req: GetBlockHeadersRequest,
    ) -> GetBlockHeadersResponse {
        bitcoin_get_block_headers(&self.pic, self.btc_id, req)
    }

    pub fn bitcoin_send_transaction(&self, req: SendTransactionRequest) {
        bitcoin_send_transaction(&self.pic, self.btc_id, req)
    }

    pub fn get_config(&self) -> Config {
        get_config(&self.pic, self.btc_id)
    }

    pub fn set_config(&self, req: SetConfigRequest) {
        set_config(&self.pic, self.btc_id, req)
    }

    /// Upgrades the bitcoin canister in place, reloading the same wasm used at
    /// install time from `IC_BTC_CANISTER_WASM_PATH`.
    pub fn upgrade_bitcoin_canister(&self, set_config: Option<SetConfigRequest>) {
        let wasm = load_wasm("IC_BTC_CANISTER_WASM_PATH", "ic-btc-canister");
        upgrade_bitcoin_canister(&self.pic, self.btc_id, wasm, set_config)
    }
}
