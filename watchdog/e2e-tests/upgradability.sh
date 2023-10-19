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
trap 'dfx stop & rm ${REFERENCE_CANISTER_NAME}.wasm.gz' EXIT SIGINT

# Get the URL of the latest release for watchdog-canister.
get_latest_release_url() {
  local page=1
  local url

  # Set a limit to the number of pages and the delay between the retries.
  local page_limit=10
  local delay_sec=2

  while [ "$page" -le "$page_limit" ]; do
    api_response=$(curl -i -s "https://api.github.com/repos/dfinity/bitcoin-canister/releases?page=$page")

    # Check if we have reached the rate limit.
    rate_limit_remaining=$(echo "$api_response" | grep -i "X-RateLimit-Remaining:" | tr -d '[:space:]' | cut -d ':' -f 2)
    if [ "$rate_limit_remaining" -le 0 ]; then
      echo "GitHub API rate limit exceeded. Please wait and try again later."
      rate_limit_reset=$(echo "$api_response" | grep -i "X-RateLimit-Reset:" | tr -d '[:space:]' | cut -d ':' -f 2)
      current_time=$(date +%s)
      time_to_reset=$((rate_limit_reset - current_time))
      echo "Rate limiting will reset at: $(date -d @$rate_limit_reset)"
      echo "You need to wait for $time_to_reset seconds from now."
      echo ""
      exit 3
    fi

    # There might be several releases on a page, but we only want the first one.
    url=$(echo "$api_response" | grep -m 1 "browser_download_url.*watchdog-canister.wasm.gz" | cut -d '"' -f 4)

    if [ -z "$url" ]; then
      echo "No release found on page $page." >/dev/null
      sleep $delay_sec
      ((page++))
    fi

    # Check if the URL points to a valid file.
    if wget --spider "$url" 2>/dev/null; then
      break
    fi
  done

  echo "$url"
}

# Download the latest release.
download_latest_release() {
  local url
  url=$(get_latest_release_url)

  if [ -n "$url" ]; then
    echo "Found watchdog-canister.wasm.gz at URL: $url"
    wget -O "${REFERENCE_CANISTER_NAME}.wasm.gz" "$url"
  else
    echo "No release with watchdog-canister.wasm.gz found at: $url"
    echo ""
    exit 2
  fi
}
download_latest_release

dfx start --background --clean

# Deploy the latest release.
dfx deploy --no-wallet ${REFERENCE_CANISTER_NAME} --argument "${ARGUMENT}"

dfx canister stop ${REFERENCE_CANISTER_NAME}

# Update the local dfx configuration to point to the 'watchdog' canister 
# in the current branch, rather than the reference canister.
sed -i'' -e 's/'${REFERENCE_CANISTER_NAME}'/watchdog/' .dfx/local/canister_ids.json

# Verify that the watchdog canister now exists and is already stopped.
if ! [[ $(dfx canister status watchdog 2>&1) == *"Status: Stopped"* ]]; then
  echo "Failed to create and stop watchdog canister."
  echo ""
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
