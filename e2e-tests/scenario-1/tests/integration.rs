use candid::{CandidType, Deserialize, Principal};
use ic_btc_canister::types::{HttpRequest, HttpResponse};
use ic_btc_interface::{
    BlockchainInfo, CanisterArg, GetBalanceRequest, GetBlockHeadersRequest,
    GetBlockHeadersResponse, GetCurrentFeePercentilesRequest, GetUtxosRequest, GetUtxosResponse,
    InitConfig, Network, NetworkInRequest,
};
use pocket_ic::{PocketIc, PocketIcBuilder, RejectCode, RejectResponse};
use scenario_1::{ADDRESS_1, ADDRESS_2, ADDRESS_5};
use serde_bytes::ByteBuf;
use std::{path::PathBuf, process::Command};

struct Setup {
    pic: PocketIc,
    btc_id: Principal,
}

impl Setup {
    fn new() -> Self {
        let source_wasm = load_wasm("E2E_SCENARIO_1_WASM_PATH", "scenario-1");
        let btc_wasm = load_wasm("IC_BTC_CANISTER_WASM_PATH", "ic-btc-canister");

        let pic = PocketIcBuilder::new().with_application_subnet().build();

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

        Self { pic, btc_id }
    }

    fn tick_until_main_chain_height(&self, target: u32, max_ticks: u32) {
        for _ in 0..max_ticks {
            self.pic.tick();
            if self.get_blockchain_info().height >= target {
                return;
            }
        }
        panic!("timed out after {max_ticks} ticks waiting for main chain height {target}");
    }

    fn get_blockchain_info(&self) -> BlockchainInfo {
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

    fn bitcoin_get_balance(&self, req: GetBalanceRequest) -> u64 {
        self.update("bitcoin_get_balance", req)
    }

    fn bitcoin_get_balance_query(&self, req: GetBalanceRequest) -> u64 {
        self.query("bitcoin_get_balance_query", req)
    }

    fn bitcoin_get_utxos(&self, req: GetUtxosRequest) -> GetUtxosResponse {
        self.update("bitcoin_get_utxos", req)
    }

    fn bitcoin_get_utxos_query(&self, req: GetUtxosRequest) -> GetUtxosResponse {
        self.query("bitcoin_get_utxos_query", req)
    }

    fn bitcoin_get_block_headers(&self, req: GetBlockHeadersRequest) -> GetBlockHeadersResponse {
        self.update("bitcoin_get_block_headers", req)
    }

    fn bitcoin_get_current_fee_percentiles(
        &self,
        req: GetCurrentFeePercentilesRequest,
    ) -> Vec<u64> {
        self.update("bitcoin_get_current_fee_percentiles", req)
    }

    fn tick_until_stable_height(&self, target: u32, max_ticks: u32) {
        for _ in 0..max_ticks {
            self.pic.tick();
            if self
                .get_stable_height()
                .map(|h| h >= target)
                .unwrap_or(false)
            {
                return;
            }
        }
        panic!("timed out after {max_ticks} ticks waiting for stable height {target}");
    }

    fn get_stable_height(&self) -> Option<u32> {
        let request = HttpRequest {
            method: "GET".to_string(),
            url: "/metrics".to_string(),
            headers: vec![],
            body: ByteBuf::new(),
        };
        let bytes = self
            .pic
            .query_call(
                self.btc_id,
                Principal::anonymous(),
                "http_request",
                candid::encode_one(request).unwrap(),
            )
            .ok()?;
        let response = candid::decode_one::<HttpResponse>(&bytes).ok()?;
        assert_eq!(
            response.status_code,
            200,
            "metrics endpoint returned {}: {}",
            response.status_code,
            String::from_utf8_lossy(&response.body)
        );
        let body = String::from_utf8(response.body.into_vec()).ok()?;
        // The metric is encoded as f64 but always a whole number; parse as f64 first
        // so this survives any encoder change that emits "3.0" instead of "3".
        // Accept both unlabeled ("stable_height N") and labeled ("stable_height{...} N")
        // forms so a future label addition doesn't silently break the match.
        body.lines()
            .find(|line| line.starts_with("stable_height ") || line.starts_with("stable_height{"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|s| s.parse::<f64>().ok())
            .map(|v| v as u32)
    }

    fn update_call_raw(
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

fn balance_req(address: &str, min_confirmations: Option<u32>) -> GetBalanceRequest {
    GetBalanceRequest {
        address: address.to_string(),
        network: NetworkInRequest::Regtest,
        min_confirmations,
    }
}

fn utxos_req(address: &str) -> GetUtxosRequest {
    GetUtxosRequest {
        address: address.to_string(),
        network: NetworkInRequest::Regtest,
        filter: None,
    }
}

#[test]
fn scenario_1() {
    let setup = Setup::new();

    // Wait until all 5 blocks have been ingested. The scenario-1 canister serves 7
    // GetSuccessors responses (one per heartbeat call); 500 ticks is a generous ceiling.
    setup.tick_until_main_chain_height(5, 500);

    let info = setup.get_blockchain_info();
    assert_eq!(
        info.height, 5,
        "expected blockchain height 5, got {}",
        info.height
    );

    // Wait for stable-block processing to complete. With stability_threshold=2 and
    // main_chain_height=5, blocks 1–3 should become stable. Stable ingestion happens
    // incrementally across heartbeats after the main chain advances, so a few dozen
    // ticks typically suffice; 200 is a generous ceiling.
    setup.tick_until_stable_height(3, 200);

    // ADDRESS_1 has no balance: it transferred everything to ADDRESS_2 in block 2.
    assert_eq!(setup.bitcoin_get_balance(balance_req(ADDRESS_1, None)), 0);
    assert_eq!(
        setup.bitcoin_get_balance_query(balance_req(ADDRESS_1, None)),
        0
    );

    // ADDRESS_2 with min_confirmations=2: block 5's spend is excluded (only 1 confirmation at
    // tip), so it still shows the 50 BTC received in block 2.
    assert_eq!(
        setup.bitcoin_get_balance(balance_req(ADDRESS_2, Some(2))),
        5_000_000_000
    );

    // ADDRESS_2 UTXOs without filter: block 5 is included so all are spent.
    assert_eq!(setup.bitcoin_get_utxos(utxos_req(ADDRESS_2)).utxos.len(), 0);
    assert_eq!(
        setup
            .bitcoin_get_utxos_query(utxos_req(ADDRESS_2))
            .utxos
            .len(),
        0
    );

    // ADDRESS_5 has 10k UTXOs (received in block 5), but responses are capped at 1000.
    assert_eq!(
        setup.bitcoin_get_utxos(utxos_req(ADDRESS_5)).utxos.len(),
        1000
    );
    assert_eq!(
        setup
            .bitcoin_get_utxos_query(utxos_req(ADDRESS_5))
            .utxos
            .len(),
        1000
    );

    // Calling query-only methods as replicated (update) calls must be rejected.
    let err = setup
        .update_call_raw("bitcoin_get_utxos_query", utxos_req(ADDRESS_5))
        .expect_err("expected replicated bitcoin_get_utxos_query to be rejected");
    assert_eq!(err.reject_code, RejectCode::CanisterReject);

    let err = setup
        .update_call_raw("bitcoin_get_balance_query", balance_req(ADDRESS_5, None))
        .expect_err("expected replicated bitcoin_get_balance_query to be rejected");
    assert_eq!(err.reject_code, RejectCode::CanisterReject);

    // ADDRESS_5 balance.
    assert_eq!(
        setup.bitcoin_get_balance(balance_req(ADDRESS_5, None)),
        5_000_000_000
    );
    assert_eq!(
        setup.bitcoin_get_balance_query(balance_req(ADDRESS_5, None)),
        5_000_000_000
    );

    // Fee percentiles smoke test. The result is intentionally not asserted; these
    // calls exist only to exercise the endpoint for profiling, matching the
    // behaviour of the original scenario-1.sh script.
    let fee_req = || GetCurrentFeePercentilesRequest {
        network: NetworkInRequest::Regtest,
    };
    setup.bitcoin_get_current_fee_percentiles(fee_req());
    setup.bitcoin_get_current_fee_percentiles(fee_req());

    // Verify block headers. The scenario-1 canister chains 5 blocks onto the genesis block,
    // so get_block_headers returns 6 headers (genesis + blocks 1–5).
    let headers_resp = setup.bitcoin_get_block_headers(GetBlockHeadersRequest {
        start_height: 0,
        end_height: None,
        network: NetworkInRequest::Regtest,
    });
    assert_eq!(headers_resp.tip_height, 5);

    // Expected headers are the raw 80-byte Bitcoin block headers, matching the blob literals
    // in scenario-1.sh. Each \xNN byte corresponds to the \NN hex escape in the Candid blobs.
    let expected_headers: Vec<Vec<u8>> = vec![
        // Genesis block header
        b"\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
          \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
          \x00\x00\x00\x00\x3b\xa3\xed\xfd\x7a\x7b\x12\xb2\x7a\xc7\x2c\x3e\
          \x67\x76\x8f\x61\x7f\xc8\x1b\xc3\x88\x8a\x51\x32\x3a\x9f\xb8\xaa\
          \x4b\x1e\x5e\x4a\xda\xe5\x49\x4d\xff\xff\x7f\x20\x02\x00\x00\x00"
            .to_vec(),
        // Block 1
        b"\x01\x00\x00\x00\x06\x22\x6e\x46\x11\x1a\x0b\x59\xca\xaf\x12\x60\
          \x43\xeb\x5b\xbf\x28\xc3\x4f\x3a\x5e\x33\x2a\x1f\xc7\xb2\xb7\x3c\
          \xf1\x88\x91\x0f\xf0\xbd\x3e\x7d\xa3\xbc\x8d\xc6\x62\x68\x28\xb3\
          \x66\x7a\x16\xba\x4e\xef\x63\x96\x6a\x68\xeb\x4d\xfd\xae\xd7\xf1\
          \x6f\x41\x97\xc8\x32\xe8\x49\x4d\xff\xff\x7f\x20\x00\x00\x00\x00"
            .to_vec(),
        // Block 2
        b"\x01\x00\x00\x00\xb5\x2a\x48\x82\x73\x2c\x0c\xe4\x6f\x9c\x91\xa3\
          \x71\xe3\xee\x7f\x33\x02\x9b\x09\x50\x2d\xaf\x59\x8e\x5e\x2d\x4e\
          \xc2\x00\x89\x56\xf2\x83\x4a\xe9\xa7\x78\xd3\x58\x67\x63\x7e\x17\
          \xb9\xf6\x75\x5e\x03\xdd\xbb\x8c\x52\x1b\x9a\xd6\x07\xb5\xbb\xab\
          \xee\xa1\x15\x33\x8a\xea\x49\x4d\xff\xff\x7f\x20\x00\x00\x00\x00"
            .to_vec(),
        // Block 3
        b"\x01\x00\x00\x00\x9d\x9d\x5d\xb6\x5e\x61\x2a\xf4\xef\x18\xe2\x50\
          \xa8\x2a\x30\x8e\xa1\xd3\x49\xeb\x96\x88\x3b\x12\x1c\x90\x52\x35\
          \x6d\x83\x10\x69\x7e\xde\xe2\x2e\x85\x73\x88\x87\xce\x80\x9e\xc6\
          \xcf\xdf\x6c\xba\x43\xcc\xee\x51\xa9\x6e\x9a\xe6\xba\xe9\x22\x71\
          \x39\xc5\xe2\x07\xe2\xec\x49\x4d\xff\xff\x7f\x20\x01\x00\x00\x00"
            .to_vec(),
        // Block 4
        b"\x01\x00\x00\x00\xc2\x34\xc0\xc4\x59\x61\x6d\x2c\x1f\xb0\xab\xa3\
          \x92\xf5\xe7\xc2\x5d\xe3\x83\x3b\x9b\x35\xa7\x41\x1c\x4e\x9d\x08\
          \x15\x27\xfd\x55\x47\xe2\xc5\x8e\x39\x9b\x85\xd6\xfc\xe6\xbc\x46\
          \x7d\x52\x1a\x5a\x6f\x54\x1f\x02\x4c\xe2\x8e\x88\x27\xcd\xe1\xe4\
          \x23\xb2\x13\x3a\x3a\xef\x49\x4d\xff\xff\x7f\x20\x02\x00\x00\x00"
            .to_vec(),
        // Block 5
        b"\x01\x00\x00\x00\x09\xca\xab\xac\x0a\xf4\x33\x86\x14\x54\x63\x62\
          \x3f\xe9\x15\x03\x2e\xec\xa0\xda\x02\x1b\x03\xa0\x48\xbe\x22\x21\
          \xfc\xd7\x49\x54\x00\x51\x6d\x88\xc9\x36\x80\x03\xbe\x61\x36\xce\
          \x35\x41\x8b\xd3\xac\x40\x9f\x1c\xab\x5c\xed\xac\x4e\xbb\x56\x33\
          \x34\x9b\xfa\xe5\x92\xf1\x49\x4d\xff\xff\x7f\x20\x01\x00\x00\x00"
            .to_vec(),
    ];
    assert_eq!(headers_resp.block_headers, expected_headers);
}
