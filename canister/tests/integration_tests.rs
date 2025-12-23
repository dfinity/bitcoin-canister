use candid::{encode_one, Encode, Principal};
use ic_btc_canister::CanisterArg;
use ic_btc_interface::InitConfig;
use ic_cdk::management_canister::CanisterId;
use pocket_ic::{ErrorCode, PocketIc, PocketIcBuilder, RejectResponse};
use std::{path::PathBuf, process::Command};

const BUILD_SCRIPT: &str = "scripts/build-canister.sh";
const WASM_PATH: &str = "target/wasm32-unknown-unknown/release/ic-btc-canister.wasm.gz";

// Executes a bash script to build the bitcoin canister wasm.
fn build_canister() {
    let output = Command::new("bash")
        .arg(get_full_path(BUILD_SCRIPT))
        .arg("ic-btc-canister")
        .output()
        .expect("Failed to execute command");

    // Check if the command was successful
    assert!(
        output.status.success(),
        "Command failed with error: {}",
        std::str::from_utf8(&output.stderr).unwrap()
    );
}

// Reads the canister wasm.
fn canister_wasm() -> Vec<u8> {
    std::fs::read(get_full_path(WASM_PATH)).unwrap()
}

fn with_bitcoin_canister<F: Fn(PocketIc, CanisterId)>(test: F) {
    println!("Building the bitcoin canister...");
    build_canister();
    println!("Done.");

    println!("Installing the bitcoin canister...");
    let pic = PocketIcBuilder::new().with_bitcoin_subnet().build();
    let canister_id = pic.create_canister();
    let wasm_bytes = canister_wasm();
    pic.install_canister(
        canister_id,
        wasm_bytes,
        Encode!(&CanisterArg::Init(InitConfig::default())).unwrap(),
        None,
    );

    // Run the test.
    test(pic, canister_id);
}

fn get_full_path(path: &str) -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("..")
        .join(path)
}

// If canbench is included in the canister, there'll be an endpoint called "has_canbench".
// This test ensures this endpoint doesn't exist.
#[test]
fn canbench_is_not_in_bitcoin_canister() {
    with_bitcoin_canister(|pic: PocketIc, canister_id: CanisterId| {
        assert_matches::assert_matches!(
            pic.update_call(
                canister_id,
                Principal::anonymous(),
                "has_canbench",
                encode_one(()).unwrap(),
            ),
            Err(RejectResponse {
                error_code: ErrorCode::CanisterMethodNotFound,
                ..
            })
        );
    });
}
