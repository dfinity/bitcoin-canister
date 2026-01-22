#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Create and install the bitcoin canister using pre-built WASM
dfx canister create bitcoin
dfx canister install bitcoin \
  --wasm "${SCRIPT_DIR}/../wasms/ic-btc-canister.wasm.gz" \
  --argument "(variant {init = record {}})"

# Check the canister's metadata section for the Candid interface.
METADATA=$(dfx canister metadata bitcoin candid:service)

# Metadata returned should match the bitcoin canister's .did file.
DIFF_OUTPUT=$(diff "$SCRIPT_DIR/../canister/candid.did" <(echo "$METADATA"))

if [ "$DIFF_OUTPUT" != "" ]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
