#!/usr/bin/env bash
#
# A test that verifies that the `set_config` endpoint works as expected.
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Configure dfx.json to use pre-built WASM
use_prebuilt_bitcoin_wasm

# Deploy the bitcoin canister.
dfx deploy --no-wallet bitcoin --argument "(variant {init = record {
  stability_threshold = opt 0;
  network = opt variant { regtest };
}})"

# The stability threshold is zero
CONFIG=$(dfx canister call bitcoin get_config --query)
if ! [[ $CONFIG == *"stability_threshold = 0"* ]]; then
  echo "FAIL"
  exit 1
fi

# Update the stability threshold.
dfx canister call bitcoin set_config '(record {
  stability_threshold = opt (17: nat);
})'

# Verify the stability threshold has been updated.
CONFIG=$(dfx canister call bitcoin get_config --query)
if ! [[ $CONFIG == *"stability_threshold = 17"* ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
