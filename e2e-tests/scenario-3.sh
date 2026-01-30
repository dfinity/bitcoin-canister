#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the canister that returns the blocks for scenario 3.
dfx deploy --no-wallet e2e-scenario-3

# Configure dfx.json to use pre-built WASM
use_prebuilt_bitcoin_wasm

# Deploy the bitcoin canister, setting the blocks_source to be the source above.
dfx deploy --no-wallet bitcoin --argument "(variant {init = record {
  stability_threshold = opt 2;
  network = opt variant { regtest };
  blocks_source = opt principal \"$(dfx canister id e2e-scenario-3)\";
}})"

# Send transaction valid transaction
TX_BYTES="blob \"\\00\\00\\00\\00\\00\\01\\00\\00\\00\\00\\00\\00\""
dfx canister call bitcoin bitcoin_send_transaction "(record {
  network = variant { regtest };
  transaction = ${TX_BYTES}
})"

# Verify the transaction was sent.
TX_BYTES_RECEIVED=$(dfx canister call e2e-scenario-3 get_last_transaction --query)
if ! [[ $TX_BYTES_RECEIVED = "($TX_BYTES)" ]]; then
  echo "FAIL"
  exit 1
fi

# Send invalid transaction.
set +e
TX_BYTES="blob \"12341234789789\""
SEND_TX_OUTPUT=$(dfx canister call bitcoin bitcoin_send_transaction "(record {
  network = variant { regtest };
  transaction = ${TX_BYTES}
})" 2>&1);
set -e

# Should result in a panic.
if [[ $SEND_TX_OUTPUT != *"MalformedTransaction"* ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
