#!/usr/bin/env bash
set -Eexuo pipefail
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
# Add root directory to the PATH
export PATH="$SCRIPT_DIR/..:$PATH"

ROOT_DIR="$SCRIPT_DIR/.."

cd "$ROOT_DIR"

# Remove downloaded didc if we run into errors.
trap 'rm $ROOT_DIR/didc $ROOT_DIR/drun && rm -r $ROOT_DIR/old/bitcoin-canister' EXIT SIGINT

get_didc_release(){
  OS=$(uname)
  ARCH=$(uname -m)
  if [ "$OS" == "Darwin" ] && [ "$ARCH" == "arm64" ]; then
    # Apple sillicon
    wget -O didc https://github.com/dfinity/candid/releases/download/2023-07-25/didc-macos
  elif [ "$OS" == "Linux" ] && [ "$ARCH" == "x86_64" ]; then
          # Linux x86 64bit
          wget -O didc https://github.com/dfinity/candid/releases/download/2023-07-25/didc-linux64
      else
          echo "Unsoported machine"
          EXIT SIGINT
  fi
}

get_drun_release() {
  OS=$(uname | tr '[:upper:]' '[:lower:]')
  URL="https://github.com/dfinity/ic/releases/download/release-2023-09-27_23-01%2Bquic/drun-x86_64-${OS}.gz"
  wget -O "drun.gz" "${URL}"
  gzip -d drun.gz
}

get_didc_release
chmod +x didc

get_drun_release 
chmod +x drun

#PR_BRANCH=$(git rev-parse --abbrev-ref HEAD)

#git checkout master

mkdir old && cd old
git clone git@github.com:dfinity/bitcoin-canister.git
cd bitcoin-canister

cargo bench --quiet 2>&1

cd "$ROOT_DIR"

#git checkout "$PR_BRANCH"

# Run cargo bench, searching for performance regressions and outputting them to a file.
LOG_FILE="$SCRIPT_DIR/benchmarking/benchmark.txt"
cargo bench --quiet 2>&1 | tee "$LOG_FILE"

set +e
NO_CHANGE=$(grep -c "No change in performance detected." "$LOG_FILE")
IMPROVED=$(grep -c "Performance has improved." "$LOG_FILE")
set -e

# Since we have 4 benchmark tests, we expect 4 appearances of "No change in performance detected."
# or "Performance has improved." otherwise, performances are degraded.
OCCURENCES=$((NO_CHANGE+IMPROVED))

if [[ $OCCURENCES != 4 ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
