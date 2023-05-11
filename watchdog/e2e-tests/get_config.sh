#!/usr/bin/env bash
#
# A test that verifies that the `get_config` endpoint works as expected.

# Source the utility functions.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the watchdog canister.
deploy_watchdog_canister_mainnet

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
    echo "FAIL: $field not found in config."
    exit 2
  fi
done

echo "SUCCESS"
