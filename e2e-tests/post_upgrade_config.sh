#!/usr/bin/env bash
#
# A test that verifies that calling post_upgrade with a set_config_request works.
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

dfx deploy --upgrade-unchanged bitcoin --argument "(variant { upgrade = opt record {
  fees = opt $FEES;
}})"

# Verify the fees have been updated.
CONFIG=$(dfx canister call bitcoin get_config --query)
if ! [[ $CONFIG == *"get_current_fee_percentiles = 123"* ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
