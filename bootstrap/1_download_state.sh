#!/usr/bin/env bash
#
# Script for downloading the Bitcoin state up to a specified block height.
set -euo pipefail

# Ensure correct usage.
if [[ $# -ne 3 ]]; then
    echo "Usage: $0 <path-to-bitcoin-dir> <block-height> <network>"
    exit 1
fi

BITCOIN_D="$1/bin/bitcoind"
HEIGHT="$2"
NETWORK="$3"

# Validate network input.
VALID_NETWORKS=("mainnet" "testnet" "testnet4")
if ! [[ " ${VALID_NETWORKS[*]} " =~ " $NETWORK " ]]; then
    echo "Error: NETWORK must be one of ${VALID_NETWORKS[*]}."
    exit 1
fi

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
cat << EOF > "$CONF_FILE"
# Stop running after reaching the given height in the main chain.
stopatheight=$HEIGHT

# Reduce storage requirements by only storing the most recent N MiB of blocks.
prune=5000

# Dummy credentials required by bitcoin-cli.
rpcuser=ic-btc-integration
rpcpassword=QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E=
rpcauth=ic-btc-integration:cdf2741387f3a12438f69092f0fdad8e\$62081498c98bee09a0dce2b30671123fa561932992ce377585e8e08bb0c11dfa
EOF

# Add network-specific configuration if necessary.
[[ "$NETWORK" == "testnet" ]] && echo "chain=test" >> "$CONF_FILE"
[[ "$NETWORK" == "testnet4" ]] && echo "chain=testnet4" >> "$CONF_FILE"

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
