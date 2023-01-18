#!/usr/bin/env bash
set -Eexuo pipefail

# Run dfx stop if we run into errors and remove the downloaded wasm.
trap "dfx stop & rm upgradability-test.wasm.gz" EXIT SIGINT

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PARENT_DIR="$(dirname "$SCRIPT_DIR")"

pushd "$PARENT_DIR"

# The URL of the latest release.
LATEST_RELEASE="$(curl -s https://api.github.com/repos/dfinity/bitcoin-canister/releases/latest | grep "browser_download_url" | awk '{ print $2 }' | sed 's/,$//' | sed 's/"//g')"
MANAGEMENT_CANISTER="aaaaa-aa"
ARGUMENT="(record { 
 stability_threshold = 2;
 network = variant { regtest };
 blocks_source = principal \"$(dfx canister id "${MANAGEMENT_CANISTER}")\";
 fees = record { 
    get_utxos_base = 0; 
    get_utxos_cycles_per_ten_instructions = 0; 
    get_utxos_maximum = 0; get_balance = 0; 
    get_balance_maximum = 0; 
    get_current_fee_percentiles = 0; 
    get_current_fee_percentiles_maximum = 0;  
    send_transaction_base =0; 
    send_transaction_per_byte = 0; 
 }; 
 syncing = variant { enabled }; 
 api_access = variant { enabled }
})"

# Download the latest release
wget -O upgradability-test.wasm.gz "${LATEST_RELEASE}"

dfx start --background --clean

# Deploy the latest release
dfx deploy --no-wallet upgradablity-test --argument "${ARGUMENT}"

dfx canister stop upgradablity-test

# replace from upgradablity-test with bitcoin in .dfx/local/canister_ids.json
# so that the canister is upgraded to the bitcoin canister of the current branch.
sed -i'' -e 's/upgradability-test/bitcoin/' .dfx/local/canister_ids.json

# Deploy upgraded canister
dfx deploy --no-wallet bitcoin --argument "${ARGUMENT}"

dfx canister start bitcoin
dfx canister stop bitcoin

# Redeploy the canister to test the pre-upgrade hook.
dfx deploy --upgrade-unchanged bitcoin --argument "${ARGUMENT}"
dfx canister start bitcoin

echo "SUCCESS"
