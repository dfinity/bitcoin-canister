#!/usr/bin/env bash
set -Eexuo pipefail

get_balance() {
    dfx canister status bitcoin 2>&1 | grep "Balance: " | awk '{ print $2 }'
}

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

INITIAL_BALANCE=100000000000

# Configure dfx.json to use pre-built WASM
use_prebuilt_bitcoin_wasm

# Deploy the bitcoin canister.
dfx deploy --no-wallet --with-cycles "$INITIAL_BALANCE" bitcoin --argument "(variant {init = record {
  network = opt variant { regtest };
  burn_cycles = opt variant { enabled };
}})"

sleep 3

# Check that cycles are burnt.
if [ "$(get_balance)" != "0" ]; then
    EXIT SIGINT
fi

echo "SUCCESS"
