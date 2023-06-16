#!/usr/bin/env bash
#
# This script tests the upgradability of a watchdog canister.
# The process follows these steps:
# - Fetches and downloads the latest release of the watchdog canister (a reference canister).
# - Deploys this reference canister on a local IC network.
# - Upgrades the reference canister to a recent 'watchdog' canister from the current branch.
# - Verifies that the 'watchdog' canister is in a 'stopped' state.
# - Tests canister upgradability by redeploying and restarting it.

set -Eexuo pipefail

# Constants.
REFERENCE_CANISTER_NAME="reference-canister"
BITCOIN_NETWORK=mainnet
BITCOIN_CANISTER_ID=ghsi2-tqaaa-aaaan-aaaca-cai
ARGUMENT="(opt record {
  bitcoin_network = variant { ${BITCOIN_NETWORK} };
  blocks_behind_threshold = 2;
  blocks_ahead_threshold = 2;
  min_explorers = 3;
  bitcoin_canister_principal = principal \"${BITCOIN_CANISTER_ID}\";
  delay_before_first_fetch_sec = 1;
  interval_between_fetches_sec = 300;
  explorers = vec {
    variant { api_blockchair_com_mainnet };
    variant { api_blockcypher_com_mainnet };
    variant { blockchain_info_mainnet };
    variant { blockstream_info_mainnet };
    variant { chain_api_btc_com_mainnet };
  };
})"

# Source the utility functions.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors and remove the downloaded wasm.
trap "dfx stop & rm ${REFERENCE_CANISTER_NAME}.wasm.gz" EXIT SIGINT

# Get the URL of the latest release.
get_latest_release_url() {
  curl -s https://api.github.com/repos/dfinity/bitcoin-canister/releases/latest | 
  grep "browser_download_url.*watchdog-canister.wasm.gz" | 
  cut -d '"' -f 4
}

# Download the latest release.
download_latest_release() {
  local url=$(get_latest_release_url)
  wget -O "${REFERENCE_CANISTER_NAME}.wasm.gz" "${url}"
}
download_latest_release

dfx start --background --clean

# Deploy the latest release.
dfx deploy --no-wallet ${REFERENCE_CANISTER_NAME} --argument "${ARGUMENT}"

dfx canister stop ${REFERENCE_CANISTER_NAME}

# Update the local dfx configuration to point to the 'watchdog' canister 
# in the current branch, rather than the 'reference-canister'.
sed -i'' -e 's/'${REFERENCE_CANISTER_NAME}'/watchdog/' .dfx/local/canister_ids.json

# Verify that the watchdog canister now exists and is already stopped.
if ! [[ $(dfx canister status watchdog 2>&1) == *"Status: Stopped"* ]]; then
  echo "Failed to create and stop watchdog canister."
  exit 1
fi

# Deploy upgraded canister.
dfx deploy --no-wallet watchdog --argument "${ARGUMENT}"

dfx canister start watchdog
dfx canister stop watchdog

# Redeploy the canister to test the pre-upgrade hook.
dfx deploy --upgrade-unchanged watchdog --argument "${ARGUMENT}"
dfx canister start watchdog

echo "SUCCESS"
