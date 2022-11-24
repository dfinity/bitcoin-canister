#!/usr/bin/env bash
set -euo pipefail

TARGET="wasm32-unknown-unknown"
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

CANISTER=$1

if ! [[ "$CANISTER" == "ic-btc-canister" || "$CANISTER" == "uploader" ]]; then
  echo "You need to provide a canister to build. Possible values {ic-btc-canister|uploader}."
  false
fi

pushd "$SCRIPT_DIR"

# NOTE: On macOS a specific version of llvm-ar and clang need to be set here.
# Otherwise the wasm compilation of rust-secp256k1 will fail.
if [ "$(uname)" == "Darwin" ]; then
  LLVM_PATH=$(brew --prefix llvm)
  # On macs we need to use the brew versions
  AR="${LLVM_PATH}/bin/llvm-ar" CC="${LLVM_PATH}/bin/clang" cargo build -p "$CANISTER" --target "$TARGET" --release
else
  cargo build -p "$CANISTER" --target "$TARGET" --release
fi

# Navigate to root directory.
cd ..

cargo install ic-wasm --version 0.2.0 --root ./target
STATUS=$?

if [ "$STATUS" -eq "0" ]; then
      ./target/bin/ic-wasm \
      ./target/$TARGET/release/canister.wasm \
      -o ./target/$TARGET/release/canister.wasm shrink
  true
else
  echo Could not install ic-wasm
  false
fi

popd