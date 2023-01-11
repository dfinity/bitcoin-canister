#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

git status

# Get current branch
CURRENT_BRANCH="$(git branch --show-current)"

# Get newest release tag
NEWEST_RELEASE_TAG="$(curl -s https://api.github.com/repos/dfinity/bitcoin-canister/releases/latest | grep "tag_name" | awk '{ print $2 }' | sed 's/,$//' | sed 's/"//g' )"

# Go the the newest release
git checkout tags/"${NEWEST_RELEASE_TAG}"

dfx start --background --clean

dfx deploy --no-wallet e2e-scenario-3

# Deploy newest release
dfx deploy --no-wallet bitcoin --argument "(record { 
 stability_threshold = 2;
 network = variant { regtest };
 blocks_source = principal \"$(dfx canister id e2e-scenario-3)\";
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

dfx canister stop bitcoin

# Move to the current branch
git checkout "${CURRENT_BRANCH}"

# Deploy upgraded canister
dfx deploy --no-wallet bitcoin --argument "(record { 
 stability_threshold = 2;
 network = variant { regtest };
 blocks_source = principal \"$(dfx canister id e2e-scenario-3)\";
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

dfx canister start bitcoin
dfx canister stop bitcoin

echo "SUCCESS"
