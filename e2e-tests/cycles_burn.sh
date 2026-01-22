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

# Create and install the bitcoin canister using pre-built WASM
dfx canister create bitcoin
dfx canister install bitcoin \
  --wasm "${SCRIPT_DIR}/../wasms/ic-btc-canister.wasm.gz" \
  --argument "(variant {init = record {
    network = opt variant { regtest };
    burn_cycles = opt variant { enabled };
  }})"
dfx ledger fabricate-cycles --canister bitcoin --cycles "$INITIAL_BALANCE"

sleep 3

# Check that cycles are burnt.
if [ "$(get_balance)" != "0" ]; then
    EXIT SIGINT
fi

echo "SUCCESS"
