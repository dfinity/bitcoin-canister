#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the canister that returns the blocks for scenario 3.
dfx deploy --no-wallet e2e-scenario-3

# Deploy the bitcoin canister, setting the blocks_source to be the source above.
dfx deploy --no-wallet bitcoin --argument "(record {
  stability_threshold = 2;
  network = variant { regtest };
  blocks_source = principal \"$(dfx canister id e2e-scenario-3)\";
  syncing = variant { enabled };
  fees = record {
    get_utxos = 0;
    get_balance = 0;
    get_current_fee_percentiles = 0;
    send_transaction_base = 0;
    send_transaction_per_byte = 0;
  }
})"

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
if [[ $SEND_TX_OUTPUT != *"Cannot decode transaction"* ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
