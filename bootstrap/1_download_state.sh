#!/usr/bin/env bash
#
# Script for downloading the bitcoin state.
set -euo pipefail

source "$(dirname "$0")/utils.sh"

BITCOIN_D=$1/bin/bitcoind
HEIGHT=$2
NETWORK=$3

validate_network "$NETWORK"

if [ -d "data" ];
then
    echo "data directory already exists."
    exit 1
fi

# Create a directory to store the blocks.
mkdir data

# Create a bitcoin.conf file that downloads blocks up to the given height.
CONF_FILE=$(mktemp)
# Stop running after reaching the given height in the main chain.
generate_config "$NETWORK" "$CONF_FILE" "stopatheight=$HEIGHT"

LOG_FILE=$(mktemp)
echo "Downloading the bitcoin blocks up to height $HEIGHT (output streamed to $LOG_FILE)
This can take several hours..."
$BITCOIN_D -conf="$CONF_FILE" -datadir="$(pwd)/data" > "$LOG_FILE"
echo "Done."

echo "Making a backup of the downloaded state in ./data_bk"
cp -r ./data ./data_bk
echo "Done."
