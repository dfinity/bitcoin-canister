#!/usr/bin/env bash
#
# Script for dumping Bitcoin block headers into a file.
set -euo pipefail

source "./utils.sh"

BITCOIN_D="$1/bin/bitcoind"
BITCOIN_CLI="$1/bin/bitcoin-cli"
NETWORK="$2"
HEIGHT="$3"
STABLE_HEIGHT=$((HEIGHT - 12))

validate_file_exists "$BITCOIN_D"
validate_file_exists "$BITCOIN_CLI"
validate_network "$NETWORK"

# Kill all background processes on exit.
trap "kill 0" EXIT

# Create a temporary bitcoin.conf file with the required settings.
CONF_FILE=$(mktemp)
generate_config "$NETWORK" "$CONF_FILE" "networkactive=0"

# Remove any previously computed block headers file.
rm -f "$BLOCK_HEADERS_FILE"

# Start bitcoind in the background with no network access.
echo "Starting bitcoind for $NETWORK..."
"$BITCOIN_D" -conf="$CONF_FILE" -datadir="$DATA_DIR" > /dev/null &
BITCOIND_PID=$!

# Wait for bitcoind to initialize.
echo "Waiting for bitcoind to load..."
sleep 30

# Function to format seconds as xxh xxm xxs.
format_time() {
    local total_seconds=$1
    local hours=$((total_seconds / 3600))
    local minutes=$(((total_seconds % 3600) / 60))
    local seconds=$((total_seconds % 60))
    printf "%02dh %02dm %02ds" "$hours" "$minutes" "$seconds"
}

# Start timer for ETA calculation.
START_TIME=$(date +%s)

# Retrieve block hashes and headers via bitcoin-cli with progress logging.
echo "Fetching block headers up to height $STABLE_HEIGHT..."
for ((height = 0; height <= STABLE_HEIGHT; height++)); do
    BLOCK_HASH=$("$BITCOIN_CLI" -conf="$CONF_FILE" -datadir="$DATA_DIR" getblockhash "$height")
    BLOCK_HEADER=$("$BITCOIN_CLI" -conf="$CONF_FILE" -datadir="$DATA_DIR" getblockheader "$BLOCK_HASH" false)

    # Append the block hash and header to the file.
    echo "$BLOCK_HASH,$BLOCK_HEADER" >> "$BLOCK_HEADERS_FILE"

    # Calculate and log progress every 100 blocks.
    if ((height % 100 == 0 || height == STABLE_HEIGHT)); then
        CURRENT_TIME=$(date +%s)
        ELAPSED_TIME=$((CURRENT_TIME - START_TIME))
        PROCESSED_COUNT=$((height + 1))
        TOTAL_COUNT=$((STABLE_HEIGHT + 1))
        PERCENTAGE=$((100 * PROCESSED_COUNT / TOTAL_COUNT))
        REMAINING_TIME=$((ELAPSED_TIME * (TOTAL_COUNT - PROCESSED_COUNT) / PROCESSED_COUNT))
        FORMATTED_ETA=$(format_time "$REMAINING_TIME")

        echo "Processed $PROCESSED_COUNT/$TOTAL_COUNT ($PERCENTAGE%) headers, ETA: $FORMATTED_ETA"
    fi
done

# Compute and display the checksum of the block headers file.
echo "Computing checksum of $BLOCK_HEADERS_FILE..."
sha256sum "$BLOCK_HEADERS_FILE"

# Clean up.
kill "$BITCOIND_PID"
wait "$BITCOIND_PID" || true
echo "Done."
