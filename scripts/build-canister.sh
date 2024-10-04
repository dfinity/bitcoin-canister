#!/usr/bin/env bash
set -euo pipefail

TARGET="wasm32-unknown-unknown"
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

CANISTER=$1
FEATURES=${2:-}
PROFILE=${3:-release}

pushd "$SCRIPT_DIR"

# NOTE: On macOS a specific version of llvm-ar and clang need to be set here.
# Otherwise the wasm compilation of rust-secp256k1 will fail.
if [ "$(uname)" == "Darwin" ]; then
  LLVM_PATH=$(brew --prefix llvm)
  export AR="${LLVM_PATH}/bin/llvm-ar"
  export CC="${LLVM_PATH}/bin/clang"
fi

# NOTE: `-p` is used rather than `--bin` due to a quirk in cargo where, if --bin
# is used, it may include features that specified by benchmarking/testing crates
# that aren't needed in production.
if [[ -z "$FEATURES" ]]; then
  # No features provided
  cargo build -p "$CANISTER" --target "$TARGET" --profile="${PROFILE}"
else
  # Features provided
  cargo build -p "$CANISTER" --target "$TARGET" --profile="${PROFILE}" --features "$FEATURES"
fi

# Navigate to root directory.
cd ..

cargo install ic-wasm --version 0.2.0 --root ./target
STATUS=$?
if [[ "$STATUS" -eq "0" ]]; then
    ./target/bin/ic-wasm \
    "./target/$TARGET/${PROFILE}/$CANISTER.wasm" \
    -o "./target/$TARGET/${PROFILE}/$CANISTER.wasm" shrink

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

gzip -n -f "./target/$TARGET/${PROFILE}/$CANISTER.wasm"

popd
