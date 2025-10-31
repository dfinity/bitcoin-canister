#!/usr/bin/env bash
#
# A test that verifies that the `/metrics` endpoint works as expected.
set -Eexuo pipefail

# Source the utility functions.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the watchdog canister.
deploy_watchdog_canister_bitcoin_mainnet

# Check that metrics page contains specific metric names.
check_metric_names

# Echo success message.
echo "SUCCESS: Metrics check completed successfully for ${0##*/}"
