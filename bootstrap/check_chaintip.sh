#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/utils.sh"

# Ensure correct usage.
if [[ $# -ne 2 ]]; then
    echo "Usage: $0 <path-to-bitcoin-dir> <network>"
    exit 1
fi

BITCOIN_D="$1/bin/bitcoind"
BITCOIN_CLI="$1/bin/bitcoin-cli"
NETWORK="$2"

validate_network "$NETWORK"

# Kill all background processes on exit.
trap "kill 0" EXIT

# Create a temporary bitcoin.conf file with the required settings.
CONF_FILE=$(mktemp)
generate_config "$NETWORK" "$CONF_FILE" "networkactive=0"

DATA_DIR="$(pwd)/data"

# Start bitcoind in the background with no network access.
echo "Starting bitcoind for $NETWORK..."
"$BITCOIN_D" -conf="$CONF_FILE" -datadir="$DATA_DIR" > /dev/null &
BITCOIND_PID=$!

# Wait for bitcoind to initialize.
echo "Waiting for bitcoind to load..."
sleep 30

# Get chain tips.
echo "Fetching chain tips for $NETWORK..."
"$BITCOIN_CLI" -conf="$CONF_FILE" -datadir="$DATA_DIR" getchaintips

# Clean up.
kill "$BITCOIND_PID"
wait "$BITCOIND_PID" || true
echo "Done."
