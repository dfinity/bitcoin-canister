#!/usr/bin/env bash
#
# A test that verifies that the `set_config` endpoint works as expected.

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the bitcoin canister.
dfx deploy --no-wallet bitcoin --argument "(record {
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
