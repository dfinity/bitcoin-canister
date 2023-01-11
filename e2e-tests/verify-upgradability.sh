#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Run dfx stop if we run into errors.
trap "dfx stop & rm newest_release.wasm" EXIT SIGINT

# Get newest release download url
NEWEST_RELEASE="$(curl -s https://api.github.com/repos/dfinity/bitcoin-canister/releases/latest | grep "browser_download_url" | awk '{ print $2 }' | sed 's/,$//' | sed 's/"//g')"

# Get newest release
wget -O newest_release.wasm.gz "${NEWEST_RELEASE}"

gunzip newest_release.wasm.gz

dfx start --background --clean

dfx deploy --no-wallet e2e-scenario-3

# Deploy newest release
dfx deploy --no-wallet bitcoin-release --argument "(record { 
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

dfx canister stop bitcoin-release

# replace from bitcoin-release to bitcoin in .dfx/local/canister_ids.json
# hence, the upgraded canister has the same CanisteID 
sed -i 's/bitcoin-release/bitcoin/' .dfx/local/canister_ids.json 

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

echo "SUCCESS"
