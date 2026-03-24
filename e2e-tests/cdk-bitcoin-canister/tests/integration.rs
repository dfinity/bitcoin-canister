use candid::Principal;
use cargo_metadata::MetadataCommand;
use ic_btc_interface::{CanisterArg, Fees, Flag, InitConfig, Network, NetworkInRequest};
use pocket_ic::common::rest::RawEffectivePrincipal;
use pocket_ic::{PocketIcBuilder, call_candid};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Once;

/// Builds the test canister binary for wasm32 and returns the compiled WASM bytes.
fn cargo_build_canister() -> Vec<u8> {
    static LOG_INIT: Once = Once::new();
    LOG_INIT.call_once(env_logger::init);

    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let cargo_toml_path = dir.join("Cargo.toml");

    let target_dir = MetadataCommand::new()
        .manifest_path(&cargo_toml_path)
        .no_deps()
        .exec()
        .expect("failed to run cargo metadata")
        .target_directory;

    let wasm_target_dir = target_dir.join("canister-build");

    let output = Command::new("cargo")
        .args([
            "build",
            "--target",
            "wasm32-unknown-unknown",
            "--bin",
            "cdk-bitcoin-canister",
            "--release",
            "--manifest-path",
            &cargo_toml_path.to_string_lossy(),
            "--target-dir",
            wasm_target_dir.as_ref(),
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("failed to locate cargo");

    assert!(output.status.success(), "failed to compile the wasm binary");

    let wasm_path = wasm_target_dir
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("cdk-bitcoin-canister.wasm");

    std::fs::read(&wasm_path).unwrap_or_else(|e| {
        panic!(
            "failed to read compiled Wasm file from {:?}: {}",
            &wasm_path, e
        )
    })
}

/// Returns the Bitcoin canister WASM bytes.
///
/// If `IC_BTC_CANISTER_WASM_PATH` is set (CI), reads from that path.
/// Otherwise, builds locally via `scripts/build-canister.sh`.
fn load_btc_canister_wasm() -> Vec<u8> {
    if let Ok(path) = std::env::var("IC_BTC_CANISTER_WASM_PATH") {
        return std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read WASM from {path}: {e}"));
    }

    let repo_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let output = Command::new("bash")
        .arg(repo_root.join("scripts/build-canister.sh"))
        .arg("ic-btc-canister")
        .current_dir(&repo_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("failed to run build-canister.sh");
    assert!(output.status.success(), "build-canister.sh failed");

    let wasm_path = repo_root.join("target/wasm32-unknown-unknown/release/ic-btc-canister.wasm.gz");
    std::fs::read(&wasm_path)
        .unwrap_or_else(|e| panic!("failed to read WASM from {:?}: {e}", wasm_path))
}

#[test]
fn test_bitcoin_canister() {
    let blocks_source = Principal::from_text("aaaaa-aa").unwrap();

    // Mainnet
    let mainnet_id = Principal::from_slice(&[0, 0, 0, 0, 1, 160, 0, 4, 1, 1]);
    let mainnet_init = CanisterArg::Init(InitConfig {
        network: Some(Network::Mainnet),
        blocks_source: Some(blocks_source),
        stability_threshold: Some(100),
        syncing: Some(Flag::Enabled),
        api_access: Some(Flag::Enabled),
        disable_api_if_not_fully_synced: Some(Flag::Enabled),
        burn_cycles: Some(Flag::Enabled),
        lazily_evaluate_fee_percentiles: Some(Flag::Enabled),
        fees: Some(Fees::mainnet()),
        watchdog_canister: None,
    });
    test_network(NetworkInRequest::Mainnet, mainnet_id, mainnet_init);

    // Testnet
    let testnet_id = Principal::from_slice(&[0, 0, 0, 0, 1, 160, 0, 1, 1, 1]);
    let testnet_init = CanisterArg::Init(InitConfig {
        network: Some(Network::Testnet),
        blocks_source: Some(blocks_source),
        stability_threshold: Some(144),
        syncing: Some(Flag::Enabled),
        api_access: Some(Flag::Enabled),
        disable_api_if_not_fully_synced: Some(Flag::Enabled),
        burn_cycles: Some(Flag::Enabled),
        lazily_evaluate_fee_percentiles: Some(Flag::Enabled),
        fees: Some(Fees::testnet()),
        watchdog_canister: None,
    });
    test_network(NetworkInRequest::Testnet, testnet_id, testnet_init);

    // Regtest
    let regtest_id = testnet_id;
    let regtest_init = CanisterArg::Init(InitConfig {
        network: Some(Network::Regtest),
        blocks_source: Some(blocks_source),
        stability_threshold: Some(144),
        syncing: Some(Flag::Enabled),
        api_access: Some(Flag::Enabled),
        disable_api_if_not_fully_synced: Some(Flag::Enabled),
        burn_cycles: Some(Flag::Enabled),
        lazily_evaluate_fee_percentiles: Some(Flag::Enabled),
        fees: Some(Fees::default()),
        watchdog_canister: None,
    });
    test_network(NetworkInRequest::Regtest, regtest_id, regtest_init);
}

fn test_network(network: NetworkInRequest, btc_id: Principal, init_arg: CanisterArg) {
    let wasm = cargo_build_canister();
    let pic = PocketIcBuilder::new()
        .with_bitcoin_subnet()
        .with_application_subnet()
        .build();
    let canister_id = pic.create_canister();
    pic.add_cycles(canister_id, 10_000_000_000_000u128);
    pic.install_canister(canister_id, wasm, vec![], None);

    let btc_canister_wasm = load_btc_canister_wasm();
    let _ = pic.create_canister_with_id(None, None, btc_id).unwrap();
    pic.add_cycles(btc_id, 10_000_000_000_000u128);
    let encoded_args = candid::encode_one(init_arg).expect("failed to encode init args");
    pic.install_canister(btc_id, btc_canister_wasm.clone(), encoded_args, None);
    let () = call_candid(
        &pic,
        canister_id,
        RawEffectivePrincipal::None,
        "execute_all_methods",
        (network,),
    )
    .unwrap();
}
