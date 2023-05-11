#!/usr/bin/env bash
#
# A test that verifies that the `get_config` endpoint works as expected.
set -Eexuo pipefail

# Source the utility functions.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the watchdog canister.
deploy_watchdog_canister_mainnet

# Check config contains all the necessary fields.
check_config_fields

# Echo success message.
echo "SUCCESS: Config check completed successfully for ${0##*/}"
