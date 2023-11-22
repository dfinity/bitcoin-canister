#!/usr/bin/env bash
set -Eexuo pipefail

get_balance() {
    dfx canister status bitcoin 2>&1 | grep "Balance: " | awk '{ print $2 }'
}

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

INITIAL_BALANCE=100000000000

# Deploy the bitcoin canister.
dfx deploy --no-wallet --with-cycles "$INITIAL_BALANCE" bitcoin --argument "(record {
  stability_threshold = 0;
  network = variant { regtest };
  blocks_source = principal \"aaaaa-aa\";
  syncing = variant { enabled };
  fees = record {
    get_utxos_base = 0;
    get_utxos_cycles_per_ten_instructions = 0;
    get_utxos_maximum = 0;
    get_balance = 0;
    get_balance_maximum = 0;
    get_current_fee_percentiles = 0;
    get_current_fee_percentiles_maximum = 0;
    send_transaction_base = 0;
    send_transaction_per_byte = 0;
  };
  api_access = variant { enabled };
  disable_api_if_not_fully_synced = variant { enabled };
  watchdog_canister = null;
})"

sleep 3

# Check that cycles are burnt.
if [ "$(get_balance)" != "0" ]; then
    EXIT SIGINT
fi

echo "SUCCESS"
