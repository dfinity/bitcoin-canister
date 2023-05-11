#!/usr/bin/env bash
#
# A test that verifies that the `health_status` endpoint works as expected.

BITCOIN_NEWTORK=mainnet
BITCOIN_CANISTER_ID=ghsi2-tqaaa-aaaan-aaaca-cai

ITERATIONS=30
DELAY_SEC=1

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

# Check health status has specific fields.
fields=(
  "height_source"
  "height_target"
  "height_diff"
  "height_status"
  "explorers"
)

health_status=$(dfx canister call watchdog health_status --query)

for field in "${fields[@]}"; do
  if ! [[ $health_status == *"$field = "* ]]; then
    echo "FAIL: $field not found in health status"
    exit 1
  fi
done

# Request health status repeatedly, break when the data is available.
has_enough_data=0
for ((i=1; i<=ITERATIONS; i++))
do
    health_status=$(dfx canister call watchdog health_status --query)

    if ! [[ $health_status == *"height_status = variant { not_enough_data }"* ]]; then
        has_enough_data=1
        break
    fi

    sleep $DELAY_SEC
done

if [ $has_enough_data -eq 0 ]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
