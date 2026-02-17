#!/usr/bin/env bash
#
# Scenario 2: Address with very large number of stable UTXOs.
# This scenario tests fetching the UTXOs of an address that has a very large number
# of UTXOs in stable blocks.
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the canister that returns the blocks for scenario 2.
dfx deploy --no-wallet e2e-scenario-2

# Configure dfx.json to use pre-built WASM
use_prebuilt_bitcoin_wasm

# Deploy the bitcoin canister, setting the blocks_source to be the source above.
dfx deploy --no-wallet bitcoin --argument "(variant {init = record {
  stability_threshold = opt 1;
  network = opt variant { regtest };
  blocks_source = opt principal \"$(dfx canister id e2e-scenario-2)\";
}})"

# Wait until the ingestion of stable blocks is complete.
wait_until_main_chain_height 4 60

# Verify the blockchain info using the query endpoint.
BLOCKCHAIN_INFO=$(dfx canister call bitcoin get_blockchain_info --query)
if ! [[ $BLOCKCHAIN_INFO == *"height = 4"* ]]; then
  echo "FAIL: Expected height 4 in blockchain info, got $BLOCKCHAIN_INFO"
  exit 1
fi

BALANCE=$(dfx canister call bitcoin bitcoin_get_balance '(record {
  network = variant { regtest };
  address = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8"
})')

if ! [[ $BALANCE = "(40_000 : nat64)" ]]; then
  echo "FAIL"
  exit 1
fi

# Verify that we are able to fetch the UTXOs of one address.
# We temporarily pause outputting the commands to the terminal as
# this command would print thousands of UTXOs.
set +x
UTXOS=$(dfx canister call bitcoin bitcoin_get_utxos '(record {
  network = variant { regtest };
  address = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8"
})')

# The address has 40k UTXOs. The first call to get_utxos should return 1,000.
if ! [[ $(num_utxos "$UTXOS") = 1000 ]]; then
  echo "FAIL"
  exit 1
fi
set -x

echo "SUCCESS"
