#!/usr/bin/env bash

# This script tests the upgradability of the bitcoin canister.
#
# The process follows these steps:
# - Fetches and downloads the latest release of the bitcoin canister (a reference canister).
# - Deploys this reference canister on a local IC network.
# - Upgrades the reference canister to a recent 'bitcoin' canister from the current branch.
# - Verifies that the 'bitcoin' canister is in a 'stopped' state.
# - Tests canister upgradability by redeploying and restarting it.

set -Eexuo pipefail

# Constants.
REFERENCE_CANISTER_NAME="upgradability-test"

# Run dfx stop if we run into errors and remove the downloaded wasm.
trap 'dfx stop & rm ${REFERENCE_CANISTER_NAME}.wasm.gz' EXIT SIGINT

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Get the URL of the latest release.
get_latest_release_url() {
  curl -s https://api.github.com/repos/dfinity/bitcoin-canister/releases/latest | 
  grep "browser_download_url.*ic-btc-canister.wasm.gz" | 
  cut -d '"' -f 4
}

# Download the latest release.
download_latest_release() {
  local url
  url=$(get_latest_release_url)
  if [[ -z "$url" ]]; then
    echo "ERROR: Failed to retrieve latest release URL"
    exit 1
  fi
  echo "Downloading from: $url"
  wget -O "${REFERENCE_CANISTER_NAME}.wasm.gz" "${url}"
  if [[ ! -f "${REFERENCE_CANISTER_NAME}.wasm.gz" ]]; then
    echo "ERROR: wasm file was not downloaded"
    exit 1
  fi
}
download_latest_release

dfx start --background --clean

# Deploy the latest release.
dfx deploy --no-wallet ${REFERENCE_CANISTER_NAME} --argument "(variant {init = record {}})"

dfx canister stop ${REFERENCE_CANISTER_NAME}

# Update the local dfx configuration to point to the 'bitcoin' canister 
# in the current branch, rather than the reference canister.
sed -i'' -e 's/'${REFERENCE_CANISTER_NAME}'/bitcoin/' .dfx/local/canister_ids.json

# Verify that the bitcoin canister now exists and is already stopped.
if ! [[ $(dfx canister status bitcoin 2>&1) == *"Status: Stopped"* ]]; then
  echo "Failed to create and stop Bitcoin canister."
  exit 1
fi

# Configure dfx.json to use pre-built WASM
use_prebuilt_bitcoin_wasm

echo "Deploy new version of canister..."
dfx deploy --no-wallet bitcoin --argument "(variant {init = record {}})"

dfx canister start bitcoin
dfx canister stop bitcoin

echo "Upgrade canister to own version..."

# Redeploy the canister to test the pre-upgrade hook.
dfx deploy --upgrade-unchanged bitcoin --argument "(variant {upgrade})"
dfx canister start bitcoin

echo "SUCCESS"
