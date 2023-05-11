#!/usr/bin/env bash
#
# A test that verifies that the `health_status` endpoint works as expected.
set -Eexuo pipefail

# Source the utility functions.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the watchdog canister.
deploy_watchdog_canister_mainnet

# Check health status has specific fields.
check_health_status_fields

# Check if health status data is available.
check_health_status_data

# Echo success message.
echo "SUCCESS: Health status check completed successfully for ${0##*/}"
