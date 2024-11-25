#!/usr/bin/env bash
#
# Script for preparing the unstable blocks file and sets the chainstate database
# to the exact height we need.
set -euo pipefail

source "$(dirname "$0")/utils.sh"

BITCOIN_D=$1/bin/bitcoind
BITCOIN_CLI=$1/bin/bitcoin-cli
HEIGHT=$2
NETWORK=$3

validate_network "$NETWORK"

# Kill all background processes on exit.
trap "kill 0" EXIT

CONF_FILE=$(mktemp)
generate_config "$NETWORK" "$CONF_FILE" "networkactive=0"

echo "Preparing the unstable blocks..."
# Run bitcoind in the background with no network access.
$BITCOIN_D -conf="$CONF_FILE" -datadir="$(pwd)/data" > /dev/null &

# Wait for bitcoind to load.
sleep 30

STABLE_HEIGHT=$((HEIGHT-12))

echo "Getting block hash at height $((STABLE_HEIGHT+1))"
BLOCK_HASH_1=$($BITCOIN_CLI -conf="$CONF_FILE" -datadir="$(pwd)/data" getblockhash $((STABLE_HEIGHT+1)))
echo "Hash: $BLOCK_HASH_1"

echo "Getting block hash at height $((STABLE_HEIGHT+2))"
BLOCK_HASH_2=$($BITCOIN_CLI -conf="$CONF_FILE" -datadir="$(pwd)/data" getblockhash $((STABLE_HEIGHT+2)))
echo "Hash: $BLOCK_HASH_2"

$BITCOIN_CLI -conf="$CONF_FILE" getblock "$BLOCK_HASH_1" 0 > unstable_blocks
$BITCOIN_CLI -conf="$CONF_FILE" getblock "$BLOCK_HASH_2" 0 >> unstable_blocks

echo "Invalidating unstable blocks..."
$BITCOIN_CLI -conf="$CONF_FILE" invalidateblock "$BLOCK_HASH_1"

echo "Computing checksum of unstable blocks..."
sha256sum unstable_blocks
echo "Done."
