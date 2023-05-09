#!/usr/bin/env bash
#
# A test that verifies that the `get_config` endpoint works as expected.

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the watchdog canister.
dfx deploy --no-wallet watchdog

# Request config.
config=$(dfx canister call watchdog get_config --query)

# Check config contains all the following fields.
config_fields=(
  "bitcoin_network"
  "blocks_behind_threshold"
  "blocks_ahead_threshold"
  "min_explorers"
  "bitcoin_canister_principal"
  "delay_before_first_fetch_sec"
  "interval_between_fetches_sec"
)

for field in "${config_fields[@]}"; do
  if ! [[ $config == *"$field = "* ]]; then
    echo "FAIL: $field not found in config"
    exit 1
  fi
done

echo "SUCCESS"
