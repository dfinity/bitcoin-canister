#!/usr/bin/env bash
#
# Script for preparing the unstable blocks file and setting the chainstate database
# to the exact height needed.
set -euo pipefail

source "./utils.sh"

BITCOIN_D="$1/bin/bitcoind"
BITCOIN_CLI="$1/bin/bitcoin-cli"
NETWORK="$2"
HEIGHT="$3"

validate_network "$NETWORK"

# Kill all background processes on exit.
trap "kill 0" EXIT

# Create a temporary bitcoin.conf file with the required settings.
CONF_FILE=$(mktemp)
generate_config "$NETWORK" "$CONF_FILE" "networkactive=0"

echo "Preparing the unstable blocks..."
# Start bitcoind in the background with no network access.
"$BITCOIN_D" -conf="$CONF_FILE" -datadir="$DATA_DIR" > /dev/null &
BITCOIND_PID=$!

# Wait for bitcoind to initialize.
echo "Waiting for bitcoind to load..."
sleep 30

STABLE_HEIGHT=$((HEIGHT - 12))

# Fetch block hashes for unstable blocks.
echo "Fetching block hash at height $((STABLE_HEIGHT + 1))..."
BLOCK_HASH_1=$("$BITCOIN_CLI" -conf="$CONF_FILE" -datadir="$DATA_DIR" getblockhash $((STABLE_HEIGHT + 1)))
echo "Hash: $BLOCK_HASH_1"

echo "Fetching block hash at height $((STABLE_HEIGHT + 2))..."
BLOCK_HASH_2=$("$BITCOIN_CLI" -conf="$CONF_FILE" -datadir="$DATA_DIR" getblockhash $((STABLE_HEIGHT + 2)))
echo "Hash: $BLOCK_HASH_2"

# Save the unstable blocks to a file.
"$BITCOIN_CLI" -conf="$CONF_FILE" -datadir="$DATA_DIR" getblock "$BLOCK_HASH_1" 0 > "$UNSTABLE_BLOCKS_FILE"
"$BITCOIN_CLI" -conf="$CONF_FILE" -datadir="$DATA_DIR" getblock "$BLOCK_HASH_2" 0 >> "$UNSTABLE_BLOCKS_FILE"
echo "Unstable blocks saved to $UNSTABLE_BLOCKS_FILE."

# Invalidate the unstable blocks.
echo "Invalidating unstable blocks..."
"$BITCOIN_CLI" -conf="$CONF_FILE" -datadir="$DATA_DIR" invalidateblock "$BLOCK_HASH_1"

# Compute checksum of the unstable blocks file.
echo "Computing checksum of unstable blocks..."
sha256sum "$UNSTABLE_BLOCKS_FILE"
echo "Done."

# Clean up.
kill "$BITCOIND_PID"
wait "$BITCOIND_PID" || true
