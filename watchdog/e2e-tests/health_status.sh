#!/usr/bin/env bash
#
# A test that verifies that the `health_status` endpoint works as expected.

# Settings.
ITERATIONS=30
DELAY_SEC=1

# Source the utility functions.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the watchdog canister.
deploy_watchdog_canister_mainnet

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
