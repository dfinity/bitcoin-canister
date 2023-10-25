#!/usr/bin/env bash
#
# Runs a benchmark using drun. The script assumes drun is available on the caller's path.
set -euo pipefail

BENCH_NAME=$1
FILE=$(mktemp)
DRUN_LINUX_SHA="7bf08d5f1c1a7cd44f62c03f8554f07aa2430eb3ae81c7c0a143a68ff52dc7f7"
DRUN_MAC_SHA="57b506d05a6f42f7461198f79f648ad05434c72f3904834db2ced30853d01a62"
DRUN_RELEASE_URL_PREFIX="https://github.com/dfinity/ic/releases/download/release-2023-09-27_23-01%2Bquic/drun-x86_64-"
DIDC_LINUX_SHA="fca09d09ccbb6f7eaf8bc8f7a732682b1e8782794bdded993182515d73d40ac6"
DIDC_MAC_SHA="d0a94cd21c3f9908fb56e27f06382e7a42cefe8ab40ee3a144930e07f6f5987b"
DIDC_LINUX_URL="https://github.com/dfinity/candid/releases/download/2023-07-25/didc-linux64"
DIDC_MAC_URL="https://github.com/dfinity/candid/releases/download/2023-07-25/didc-macos"

CURR_DIR=$(pwd)
export PATH="$CURR_DIR:$PATH"

download_didc(){
  OS=$(uname)
  ARCH=$(uname -m)
  if [ "$OS" == "Darwin" ]; then
    wget -O didc "$DIDC_MAC_URL"
  elif [ "$OS" == "Linux" ] && [ "$ARCH" == "x86_64" ]; then
      # Linux x86 64bit
      wget -O didc "$DIDC_LINUX_URL"
    else
      echo "Unsupported machine"
      EXIT SIGINT
  fi
  chmod +x didc
  alias didc="$CURR_DIR/didc"
}

get_correct_didc_release(){
  OS=$(uname)

  if ! type "didc" > /dev/null; then
    download_didc
  else
    DIDC_LOCATION=$(type "didc" | awk '{print $3}')
    DIDC_SHA=$(shasum -a 256 "$DIDC_LOCATION" | awk '{ print $1 }')
    # Check if didc exists and if the correct version is used.
    if ! [[ "$OS" == "Linux" && "$DIDC_SHA" == "$DIDC_LINUX_SHA" ]]; then
      if ! [[ "$OS" == "Darwin" && "$DIDC_SHA" == "$DIDC_MAC_SHA" ]]; then
        download_didc
      fi
    fi
  fi
}

download_drun(){
  OS=$1
  wget -O "drun.gz" "${DRUN_RELEASE_URL_PREFIX}${OS}.gz"
  gzip -fd drun.gz
  chmod +x drun
}

get_correct_drun_release() {
  OS=$(uname | tr '[:upper:]' '[:lower:]')
  
  # Check if drun exists in current repo.
  if ! [ -e "drun" ]; then
    download_drun "$OS"
  else 
    DRUN_SHA=$(shasum -a 256 "drun" | awk '{ print $1 }')
    # Check if drun exists and if the correct version is used.
    if ! [[ "$OS" == "linux" && "$DRUN_SHA" == "$DRUN_LINUX_SHA" ]]; then
      if ! [[ "$OS" == "darwin" && "$DRUN_SHA" == "$DRUN_MAC_SHA" ]]; then
        download_drun "$OS"
      fi
    fi
  fi
}

#get_correct_didc_release

get_correct_drun_release

cat > "$FILE" << EOF
create
install rwlgt-iiaaa-aaaaa-aaaaa-cai ../target/wasm32-unknown-unknown/release/benchmarks.wasm.gz ""
query rwlgt-iiaaa-aaaaa-aaaaa-cai ${BENCH_NAME} "DIDL\x00\x00"
EOF

# Run the benchmarks, decode the output.
./drun "$FILE" --instruction-limit 99999999999999 \
    | awk '{ print $3 }' \
    | grep "44.*" -o
