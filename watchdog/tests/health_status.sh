#!/usr/bin/env bash
#
# A test that verifies that the `health_status` endpoint works as expected.

ITERATIONS=30
DELAY_SEC=1

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the watchdog canister.
dfx deploy --no-wallet watchdog

# Request health status repeatedly, break when the data is available.
has_enough_data=0
for ((i=1; i<=$ITERATIONS; i++))
do
    health_status=$(dfx canister call watchdog health_status --query)

    if ! [[ $health_status == *"status = variant { not_enough_data }"* ]]; then
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
