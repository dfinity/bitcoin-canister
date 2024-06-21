#!/usr/bin/env bash
set -euo pipefail

TARGET="wasm32-unknown-unknown"
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

CANISTER=$1
FEATURES=${2:-}

pushd "$SCRIPT_DIR"

# NOTE: On macOS a specific version of llvm-ar and clang need to be set here.
# Otherwise the wasm compilation of rust-secp256k1 will fail.
if [ "$(uname)" == "Darwin" ]; then
  LLVM_PATH=$(brew --prefix llvm)
  export AR="${LLVM_PATH}/bin/llvm-ar"
  export CC="${LLVM_PATH}/bin/clang"
fi

if [[ -z "$FEATURES" ]]; then
  # No features provided
  cargo build --bin "$CANISTER" --target "$TARGET" --release
else
  # Features provided
  cargo build --bin "$CANISTER" --target "$TARGET" --release --features "$FEATURES"
fi

# Navigate to root directory.
cd ..

cargo install ic-wasm --version 0.2.0 --root ./target
STATUS=$?
if [[ "$STATUS" -eq "0" ]]; then
    ./target/bin/ic-wasm \
    "./target/$TARGET/release/$CANISTER.wasm" \
    -o "./target/$TARGET/release/$CANISTER.wasm" shrink

    if [[ "$CANISTER" == "ic-btc-canister" ]]; then
    	./target/bin/ic-wasm \
    	"./target/$TARGET/release/$CANISTER.wasm" \
    	-o "./target/$TARGET/release/$CANISTER.wasm" \
    	metadata candid:service -f "$SCRIPT_DIR/../canister/candid.did" -v public
    fi

  true
else
  echo Could not install ic-wasm
  false
fi

gzip -n -f "./target/$TARGET/release/$CANISTER.wasm"

popd
