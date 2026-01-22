#!/usr/bin/env bash
#
# A test that verifies that calling post_upgrade with a set_config_request works.
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Copy pre-built WASM to expected location and deploy
mkdir -p "${SCRIPT_DIR}/../target/wasm32-unknown-unknown/release"
cp "${SCRIPT_DIR}/../wasms/ic-btc-canister.wasm.gz" \
   "${SCRIPT_DIR}/../target/wasm32-unknown-unknown/release/ic-btc-canister.wasm.gz"

dfx deploy --no-wallet --no-build bitcoin --argument "(variant {init = record {
  stability_threshold = opt 0;
  network = opt variant { regtest };
}})"

# The stability threshold is zero
CONFIG=$(dfx canister call bitcoin get_config --query)
if ! [[ $CONFIG == *"stability_threshold = 0"* ]]; then
  echo "FAIL"
  exit 1
fi

# Upgrade and update the fees.
FEES="record {
  get_current_fee_percentiles = 123 : nat;
  get_utxos_maximum = 0 : nat;
  get_block_headers_cycles_per_ten_instructions = 0 : nat;
  get_current_fee_percentiles_maximum = 0 : nat;
  send_transaction_per_byte = 0 : nat;
  get_balance = 0 : nat;
  get_utxos_cycles_per_ten_instructions = 0 : nat;
  get_block_headers_base = 0 : nat;
  get_utxos_base = 0 : nat;
  get_balance_maximum = 0 : nat;
  send_transaction_base = 0 : nat;
  get_block_headers_maximum = 0 : nat;
}";

dfx deploy --no-wallet --no-build --upgrade-unchanged bitcoin --argument "(variant { upgrade = opt record {
  fees = opt $FEES;
}})"

# Verify the fees have been updated.
CONFIG=$(dfx canister call bitcoin get_config --query)
if ! [[ $CONFIG == *"get_current_fee_percentiles = 123"* ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
