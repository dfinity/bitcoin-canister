#!/usr/bin/env bash
#
# A test that verifies that the `/metrics` endpoint works as expected.

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the watchdog canister.
dfx deploy --no-wallet watchdog

# Request canister id.
CANISTER_ID=$(dfx canister id watchdog)
METRICS=$(curl "http://127.0.0.1:8000/metrics?canisterId=$CANISTER_ID")

# Check that metrics report contains some information.
if ! [[ "$METRICS" == *"bitcoin_canister_height"* ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
