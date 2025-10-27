#!/usr/bin/env bash
#
# This script tests the upgradability of the watchdog canister.
# The process follows these steps:
# - Fetches and downloads the latest release of the watchdog canister (a reference canister).
# - Deploys this reference canister on a local IC network.
# - Upgrades the reference canister to a recent 'watchdog' canister from the current branch.
# - Verifies that the 'watchdog' canister is in a 'stopped' state.
# - Tests canister upgradability by redeploying and restarting it.

set -Eexuo pipefail

# Constants.
REFERENCE_CANISTER_NAME="watchdog-upgradability-test"
ARGUMENT="(variant { bitcoin_mainnet })"

# Source the utility functions.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors and remove the downloaded wasm.
trap 'dfx stop & rm ${REFERENCE_CANISTER_NAME}.wasm.gz' EXIT SIGINT

# Get the URL of the latest release for watchdog-canister.
get_latest_release_url() {
  local page=1
  local url
  local page_limit=10
  local delay_sec=2

  while [ "$page" -le "$page_limit" ]; do
    api_response=$(curl -i -s "https://api.github.com/repos/dfinity/bitcoin-canister/releases?page=$page")

    # Check rate limit and calculate time to reset.
    rate_limit_remaining=$(grep -i "X-RateLimit-Remaining:" <<< "$api_response" | tr -d '[:space:]' | cut -d ':' -f 2)
    if [ "$rate_limit_remaining" -le 0 ]; then
      rate_limit_reset=$(grep -i "X-RateLimit-Reset:" <<< "$api_response" | tr -d '[:space:]' | cut -d ':' -f 2)
      current_time=$(date +%s)
      time_to_reset=$((rate_limit_reset - current_time))
      echo "GitHub API rate limit exceeded. Please wait and try again later."
      echo "Rate limiting will reset at: $(date -d @"$rate_limit_reset")"
      echo "You need to wait for $time_to_reset seconds from now."
      exit 2
    fi

    # Extract the URL of the first release.
    url=$(grep -m 1 "browser_download_url.*watchdog-canister.wasm.gz" <<< "$api_response" | cut -d '"' -f 4)

    if [ -n "$url" ]; then
      echo "$url"
      return
    fi

    sleep $delay_sec
    ((page++))
  done

  echo "No release found after $page_limit pages."
  exit 3
}

# Download the latest release.
download_latest_release() {
  local url
  url=$(get_latest_release_url)

  if [ -n "$url" ]; then
    echo "Found watchdog-canister.wasm.gz at URL: $url"
    if wget -O "${REFERENCE_CANISTER_NAME}.wasm.gz" "$url"; then
      echo "Download successful."
    else
      echo "Download failed. Please check the URL or try again later."
      exit 4
    fi
  fi
}
download_latest_release

dfx start --background --clean

# Deploy the latest release.
# TODO (mducroux): The new watchdog canister currently expects 'bitcoin_mainnet' as its argument, whereas the previous
# TODO (mducroux): release uses 'mainnet'. Update this line to use "${ARGUMENT}" in the next release.
dfx deploy --no-wallet ${REFERENCE_CANISTER_NAME} --argument "mainnet"

dfx canister stop ${REFERENCE_CANISTER_NAME}

# Update the local dfx configuration to point to the 'watchdog' canister 
# in the current branch, rather than the reference canister.
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
