#!/usr/bin/env bash
set -Eexuo pipefail
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
# Add directory to the PATH
export PATH="$SCRIPT_DIR:$PATH"

# Remove downloaded didc if we run into errors.
trap 'rm didc' EXIT SIGINT

# NOTE: On macOS a specific version of llvm-ar and clang need to be set here.
# Otherwise the wasm compilation of rust-secp256k1 will fail.
if [ "$(uname)" == "Darwin" ]; then
  LLVM_PATH=$(brew --prefix llvm)
  export AR="${LLVM_PATH}/bin/llvm-ar"
  export CC="${LLVM_PATH}/bin/clang"
  wget -O didc https://github.com/dfinity/candid/releases/download/2023-07-25/didc-macos
elif [ "$(uname)" == "Linux" ]; then
        wget -O didc https://github.com/dfinity/candid/releases/download/2023-07-25/didc-linux64
    else
        echo "Unsoported machine"
        EXIT SIGINT
fi

chmod +x didc

# Run cargo bench, searching for performance regressions and outputting them to a file.
LOG_FILE="$SCRIPT_DIR"/benchmarking/output.txt
cargo bench 2>&1 | tee "$LOG_FILE"
#sed -n 'No change in performance detected.' "$LOG_FILE" > "$BECNHMARK_OUT_FILE"