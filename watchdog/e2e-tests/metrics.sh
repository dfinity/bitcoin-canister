#!/usr/bin/env bash
#
# A test that verifies that the `/metrics` endpoint works as expected.

BITCOIN_NEWTORK=mainnet
BITCOIN_CANISTER_ID=ghsi2-tqaaa-aaaan-aaaca-cai

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the watchdog canister.
dfx deploy --no-wallet watchdog --argument "(record {
    bitcoin_network = variant { ${BITCOIN_NEWTORK} };
    blocks_behind_threshold = 2;
    blocks_ahead_threshold = 2;
    min_explorers = 2;
    bitcoin_canister_principal = principal \"${BITCOIN_CANISTER_ID}\";
    delay_before_first_fetch_sec = 1;
    interval_between_fetches_sec = 60;
})"

# Request canister id.
CANISTER_ID=$(dfx canister id watchdog)
METRICS=$(curl "http://127.0.0.1:8000/metrics?canisterId=$CANISTER_ID")

# Check that metrics page contains specific metric names.
metric_names=(
  "bitcoin_network"
  "blocks_behind_threshold"
  "blocks_ahead_threshold"
  "min_explorers"
  "bitcoin_canister_height"
  "height_target"
  "height_diff"
  "height_status"
  "api_access_target"
  "explorer_height"
  "available_explorers"
)

for name in "${metric_names[@]}"; do
  if ! [[ $METRICS == *"$name"* ]]; then
    echo "FAIL: $name not found in metrics page"
    exit 1
  fi
done

echo "SUCCESS"
