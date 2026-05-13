use candid::{CandidType, Deserialize, Principal};
use ic_btc_interface::{
    BlockchainInfo, CanisterArg, GetBalanceRequest, GetBlockHeadersRequest,
    GetBlockHeadersResponse, GetCurrentFeePercentilesRequest, GetUtxosRequest, GetUtxosResponse,
    InitConfig, Network,
};
use pocket_ic::{PocketIc, PocketIcBuilder, RejectResponse};
use std::{path::PathBuf, process::Command};

pub struct Setup {
    pub pic: PocketIc,
    pub source_id: Principal,
    pub btc_id: Principal,
}

impl Setup {
    pub fn new() -> Self {
        let source_wasm = load_wasm("E2E_SCENARIO_1_WASM_PATH", "scenario-1");
        let btc_wasm = load_wasm("IC_BTC_CANISTER_WASM_PATH", "ic-btc-canister");

        let pic = PocketIcBuilder::new().with_bitcoin_subnet().build();

        let source_id = pic.create_canister();
        pic.add_cycles(source_id, 10_000_000_000_000);
        pic.install_canister(source_id, source_wasm, vec![], None);

        let btc_id = pic.create_canister();
        pic.add_cycles(btc_id, 10_000_000_000_000);
        pic.install_canister(
            btc_id,
            btc_wasm,
            candid::encode_one(CanisterArg::Init(InitConfig {
                stability_threshold: Some(2),
                network: Some(Network::Regtest),
                blocks_source: Some(source_id),
                ..Default::default()
            }))
            .unwrap(),
            None,
        );

        Self {
            pic,
            source_id,
            btc_id,
        }
    }

    pub fn tick(&self) {
        self.pic.tick();
    }

    pub fn tick_until_main_chain_height(&self, target: u32, max_ticks: u32) {
        for _ in 0..max_ticks {
            self.pic.tick();
            let reached = self
                .pic
                .query_call(
                    self.btc_id,
                    Principal::anonymous(),
                    "get_blockchain_info",
                    candid::encode_args(()).unwrap(),
                )
                .ok()
                .and_then(|b| candid::decode_one::<BlockchainInfo>(&b).ok())
                .map(|info| info.height >= target)
                .unwrap_or(false);
            if reached {
                return;
            }
        }
        panic!("timed out after {max_ticks} ticks waiting for main chain height {target}");
    }

    pub fn get_blockchain_info(&self) -> BlockchainInfo {
        let bytes = self
            .pic
            .query_call(
                self.btc_id,
                Principal::anonymous(),
                "get_blockchain_info",
                candid::encode_args(()).unwrap(),
            )
            .expect("get_blockchain_info query failed");
        candid::decode_one(&bytes).expect("failed to decode BlockchainInfo")
    }

    pub fn bitcoin_get_balance(&self, req: GetBalanceRequest) -> u64 {
        self.update("bitcoin_get_balance", req)
    }

    pub fn bitcoin_get_balance_query(&self, req: GetBalanceRequest) -> u64 {
        self.query("bitcoin_get_balance_query", req)
    }

    pub fn bitcoin_get_utxos(&self, req: GetUtxosRequest) -> GetUtxosResponse {
        self.update("bitcoin_get_utxos", req)
    }

    pub fn bitcoin_get_utxos_query(&self, req: GetUtxosRequest) -> GetUtxosResponse {
        self.query("bitcoin_get_utxos_query", req)
    }

    pub fn bitcoin_get_block_headers(
        &self,
        req: GetBlockHeadersRequest,
    ) -> GetBlockHeadersResponse {
        self.update("bitcoin_get_block_headers", req)
    }

    pub fn bitcoin_get_current_fee_percentiles(
        &self,
        req: GetCurrentFeePercentilesRequest,
    ) -> Vec<u64> {
        self.update("bitcoin_get_current_fee_percentiles", req)
    }

    /// Makes an update call and returns the raw result, including any rejection.
    /// Use this to test that a method rejects when called in replicated mode.
    pub fn update_call_raw(
        &self,
        method: &str,
        arg: impl CandidType,
    ) -> Result<Vec<u8>, RejectResponse> {
        self.pic.update_call(
            self.btc_id,
            Principal::anonymous(),
            method,
            candid::encode_one(arg).unwrap(),
        )
    }

    fn query<T: CandidType + for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        arg: impl CandidType,
    ) -> T {
        let bytes = self
            .pic
            .query_call(
                self.btc_id,
                Principal::anonymous(),
                method,
                candid::encode_one(arg).unwrap(),
            )
            .unwrap_or_else(|e| panic!("{method} query failed: {e:?}"));
        candid::decode_one(&bytes).unwrap_or_else(|e| panic!("decode {method} response: {e}"))
    }

    fn update<T: CandidType + for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        arg: impl CandidType,
    ) -> T {
        let bytes = self
            .pic
            .update_call(
                self.btc_id,
                Principal::anonymous(),
                method,
                candid::encode_one(arg).unwrap(),
            )
            .unwrap_or_else(|e| panic!("{method} update call failed: {e:?}"));
        candid::decode_one(&bytes).unwrap_or_else(|e| panic!("decode {method} response: {e}"))
    }
}

fn load_wasm(env_var: &str, canister_name: &str) -> Vec<u8> {
    if let Ok(path) = std::env::var(env_var) {
        return std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read WASM from {path}: {e}"));
    }
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("bash")
        .arg(repo_root.join("scripts/build-canister.sh"))
        .arg(canister_name)
        .current_dir(&repo_root)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn build-canister.sh for {canister_name}: {e}"));
    assert!(
        output.status.success(),
        "build-canister.sh {canister_name} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let wasm_path = repo_root.join(format!(
        "target/wasm32-unknown-unknown/release/{canister_name}.wasm.gz"
    ));
    std::fs::read(&wasm_path)
        .unwrap_or_else(|e| panic!("failed to read WASM from {wasm_path:?}: {e}"))
}
