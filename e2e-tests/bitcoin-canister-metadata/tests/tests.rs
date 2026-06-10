//! Verifies the `candid:service` metadata embedded in the bitcoin canister
//! wasm matches the committed `canister/candid.did`.
//!
//! The original `bitcoin-canister-metadata.sh` deployed the canister with dfx
//! and read the section via `dfx canister metadata bitcoin candid:service`.
//! PocketIC 11 exposes no API to read a canister's metadata custom sections, so
//! we read the `icp:public candid:service` section straight from the wasm — the
//! same build-artifact property the shell test checked, without the dfx
//! round-trip.

use std::io::Read;
use std::path::PathBuf;

/// The wasm custom section that dfx exposes as the `candid:service` metadata.
/// `icp:public` is the IC naming convention for a publicly-readable section.
const CANDID_SERVICE_SECTION: &str = "icp:public candid:service";

#[test]
fn candid_service_metadata_matches_did_file() {
    let wasm = maybe_gunzip(e2e_test_utils::load_wasm(
        "IC_BTC_CANISTER_WASM_PATH",
        "ic-btc-canister",
    ));

    let metadata = candid_service_section(&wasm).expect(
        "bitcoin canister wasm has no `icp:public candid:service` metadata section. \
         This section is embedded by `scripts/build-canister.sh` (via `ic-wasm metadata`), \
         not by a plain `cargo build`, so a raw locally-built wasm won't have it. \
         Point `IC_BTC_CANISTER_WASM_PATH` at a `build-canister.sh`-produced wasm.",
    );

    let did_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../canister/candid.did");
    let expected = std::fs::read(&did_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", did_path.display()));

    assert_eq!(
        metadata,
        expected,
        "candid:service metadata embedded in the wasm does not match {}",
        did_path.display(),
    );
}

/// Decompresses `bytes` if they carry the gzip magic header, otherwise returns
/// them unchanged. The prebuilt CI artifact is `.wasm.gz`; the local escargot
/// fallback in `load_wasm` produces a raw `.wasm`.
fn maybe_gunzip(bytes: Vec<u8>) -> Vec<u8> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&bytes[..])
            .read_to_end(&mut out)
            .expect("failed to gunzip bitcoin canister wasm");
        out
    } else {
        bytes
    }
}

/// Extracts the `icp:public candid:service` custom section payload from a wasm
/// module, or `None` if it is absent.
fn candid_service_section(wasm: &[u8]) -> Option<Vec<u8>> {
    for payload in wasmparser::Parser::new(0).parse_all(wasm) {
        if let wasmparser::Payload::CustomSection(reader) =
            payload.expect("failed to parse bitcoin canister wasm")
        {
            if reader.name() == CANDID_SERVICE_SECTION {
                return Some(reader.data().to_vec());
            }
        }
    }
    None
}
