#!/usr/bin/env bash
#
# TODO: Add a description of the test.
set -Eexuo pipefail

# Source the utility functions.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy fake bitcoin canister.
dfx deploy --no-wallet watchdog-e2e-fake-bitcoin-canister
BITCOIN_CANISTER_ID=$(dfx canister id watchdog-e2e-fake-bitcoin-canister)
if [[ -z "${BITCOIN_CANISTER_ID}" ]]; then
  echo "Failed to create bitcoin canister"
  exit 1
fi

METRICS=$(curl "http://127.0.0.1:8000/metrics?canisterId=$BITCOIN_CANISTER_ID")
echo "$METRICS"

# Deploy the watchdog canister.
dfx deploy --no-wallet watchdog --argument "(opt record {
  bitcoin_network = variant { mainnet };
  blocks_behind_threshold = 2;
  blocks_ahead_threshold = 2;
  min_explorers = 2;
  bitcoin_canister_principal = principal \"${BITCOIN_CANISTER_ID}\";
  delay_before_first_fetch_sec = 1;
  interval_between_fetches_sec = 60;
  use_fake_bitcoin_canister = true;
})"

# Check if health status data is available.
check_health_status_data

config=$(dfx canister call watchdog get_config --query)
echo "$config"

health_status=$(dfx canister call watchdog health_status --query)
echo "$health_status"

sleep 30
