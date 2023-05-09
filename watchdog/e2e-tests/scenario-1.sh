#!/usr/bin/env bash
set -Eexuo pipefail

# Source the utility functions.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

# TODO: start fake explorers server.

# Start the local dfx.
dfx start --background --clean

# Create watchdog canister and save its id.
# TODO: check if --with-cycles is needed.
dfx canister create --no-wallet watchdog

WATCHDOG_CANISTER_ID=$(dfx canister id watchdog)

# TODO: Create fake bitcoin canister and save its id.
# dfx canister create --no-wallet fake_bitcoin_canister
# BITCOIN_CANISTER_ID=$(dfx canister id fake_bitcoin_canister)

BITCOIN_CANISTER_ID=g4xu7-jiaaa-aaaan-aaaaq-cai # TODO: remove debug value.

# Deploy watchdog canister.
dfx deploy --no-wallet watchdog --argument "(record {
    bitcoin_network = variant { mainnet };
    blocks_behind_threshold = 2;
    blocks_ahead_threshold = 2;
    min_explorers = 2;
    bitcoin_canister_principal = principal \"${BITCOIN_CANISTER_ID}\";
    delay_before_first_fetch_sec = 1;
    interval_between_fetches_sec = 60;
})"

CONFIG=$(dfx canister call watchdog get_config)
echo "CONFIG: ${CONFIG}"

API_ACCESS_TARGET=$(dfx canister call watchdog get_api_access_target)
echo "API_ACCESS_TARGET: ${API_ACCESS_TARGET}"

# TODO: Deploy fake bitcoin canister.

# Wait until watchdog fetches the data.
sleep 3

# Check watchdog API access target.
API_ACCESS_TARGET=$(dfx canister call watchdog get_api_access_target)
echo "API_ACCESS_TARGET: ${API_ACCESS_TARGET}"
