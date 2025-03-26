#!/bin/bash

set -e

# Generate date-time prefix: e.g. 20250326_103012
PREFIX=$(date +"%Y%m%d_%H%M%S")
BASE="./unstable_blocks/${PREFIX}_output"

echo "Fetching unstable blocks to ${BASE}.txt ..."
dfx canister call --network testnet bitcoin_t get_unstable_blocks > "${BASE}.txt"

echo "Parsing ${BASE}.txt to ${BASE}.json ..."
./unstable_blocks.py "${BASE}.txt" "${BASE}.json"

echo "Generating blockchain graph to ${BASE}.png ..."
./visualize_blockchain_graphviz.py "${BASE}.json" "${BASE}.png"
