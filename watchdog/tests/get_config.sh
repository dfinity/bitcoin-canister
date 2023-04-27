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

# Check that the config is correct, eg. by checking it has min_explores field.
if ! [[ $config == *"min_explorers = "* ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
