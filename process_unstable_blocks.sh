#!/bin/bash

set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <suffix>"
  exit 1
fi

SUFFIX="$1"
BASE="./unstable_blocks/output_${SUFFIX}"

echo "Fetching unstable blocks to ${BASE}.txt ..."
dfx canister call --network testnet bitcoin_t get_unstable_blocks > "${BASE}.txt"

echo "Parsing ${BASE}.txt to ${BASE}.json ..."
./unstable_blocks.py "${BASE}.txt" "${BASE}.json"

echo "Generating blockchain graph to blockchain_${SUFFIX}.png ..."
./visualize_blockchain_graphviz.py "${BASE}.json" "blockchain_${SUFFIX}.png"
