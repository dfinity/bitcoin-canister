#!/usr/bin/env bash
#
# Runs a benchmark using drun. The script assumes drun is available on the caller's path.
set -euo pipefail

# Remove downloaded didc, drun.
trap 'set +e && rm drun didc && set -e' EXIT SIGINT

BENCH_NAME=$1
FILE=$(mktemp)

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Add script directory to the PATH
export PATH="$SCRIPT_DIR:$PATH"

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

if ! type "didc" > /dev/null; then
  get_didc_release
  chmod +x didc
fi

if ! type "drun" > /dev/null; then
  get_drun_release 
  chmod +x drun
fi

cat > "$FILE" << EOF
create
install rwlgt-iiaaa-aaaaa-aaaaa-cai ../target/wasm32-unknown-unknown/release/benchmarks.wasm.gz ""
query rwlgt-iiaaa-aaaaa-aaaaa-cai ${BENCH_NAME} "DIDL\x00\x00"
EOF

# Run the benchmarks, decode the output.
drun "$FILE" --instruction-limit 99999999999999 \
    | awk '{ print $3 }' \
    | grep "44.*" -o
