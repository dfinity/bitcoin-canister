#!/usr/bin/env bash
#
# Script for downloading the Bitcoin state up to a specified block height.
set -euo pipefail

source "$(dirname "$0")/utils.sh"

# Ensure correct usage.
if [[ $# -ne 3 ]]; then
    echo "Usage: $0 <path-to-bitcoin-dir> <block-height> <network>"
    exit 1
fi

BITCOIN_D="$1/bin/bitcoind"
HEIGHT="$2"
NETWORK="$3"

validate_network "$NETWORK"

# Check if the data directory already exists.
DATA_DIR="$(pwd)/data"
if [[ -d "$DATA_DIR" ]]; then
    echo "Error: The 'data' directory already exists. Please remove it or choose another directory."
    exit 1
fi

# Create the data directory.
mkdir "$DATA_DIR"

# Generate a temporary bitcoin.conf file with required settings.
CONF_FILE=$(mktemp)
# Stop running after reaching the given height in the main chain.
generate_config "$NETWORK" "$CONF_FILE" "stopatheight=$HEIGHT"

# Log file for monitoring progress.
LOG_FILE=$(mktemp)
echo "Downloading Bitcoin blocks up to height $HEIGHT. Logs can be found in: $LOG_FILE"
echo "This may take several hours. Please wait..."

# Start the Bitcoin daemon.
"$BITCOIN_D" -conf="$CONF_FILE" -datadir="$DATA_DIR" > "$LOG_FILE" 2>&1
echo "Download complete."

# Create a backup of the downloaded data.
BACKUP_DIR="./data_bk"
echo "Creating a backup of the downloaded state in: $BACKUP_DIR"
cp -r "$DATA_DIR" "$BACKUP_DIR"
echo "Backup complete."
