#!/usr/bin/env bash
set -euo pipefail

TARGET="wasm32-unknown-unknown"
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

CANISTER=$1

pushd "$SCRIPT_DIR"

# NOTE: On macOS a specific version of llvm-ar and clang need to be set here.
# Otherwise the wasm compilation of rust-secp256k1 will fail.
if [ "$(uname)" == "Darwin" ]; then
  LLVM_PATH=$(brew --prefix llvm)
  export AR="${LLVM_PATH}/bin/llvm-ar"
  export CC="${LLVM_PATH}/bin/clang"
fi

cargo build --bin "$CANISTER" --target "$TARGET" --release

# Navigate to root directory.
cd ..

# If we're building the bitcoin canister, then shrink it.
# We don't shrink other canisters as installing ic-wasm in CI can take quite some time.
if [[ "$CANISTER" == "bitcoin-canister" ]]; then
  cargo install ic-wasm --version 0.2.0 --root ./target
  STATUS=$?
  if [[ "$STATUS" -eq "0" ]]; then
      ./target/bin/ic-wasm \
      "./target/$TARGET/release/$CANISTER.wasm" \
      -o "./target/$TARGET/release/$CANISTER.wasm" shrink
    true
  else
    echo Could not install ic-wasm
    false
  fi
fi

popd
