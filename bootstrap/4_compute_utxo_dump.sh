#!/usr/bin/env bash
#
# A script for building the UTXO dump text file.
set -euo pipefail

source "./utils.sh"

NETWORK=$1

validate_network "$NETWORK"

# Determine the chainstate directory based on the network.
if [[ "$NETWORK" == "mainnet" ]]; then
    CHAIN_STATE_DIR=$DATA_DIR/chainstate
elif [[ "$NETWORK" == "testnet" ]]; then
    CHAIN_STATE_DIR=$DATA_DIR/testnet3/chainstate
else
    echo "Error: unknown network $NETWORK, can't define CHAIN_STATE_DIR."
    exit 1
fi

echo "Generating the UTXO dump for $NETWORK..."
~/go/bin/bitcoin-utxo-dump -db "$CHAIN_STATE_DIR" -o "$UTXO_DUMP" -f "height,txid,vout,amount,type,address,script,coinbase,nsize"

echo "Removing the headers from the file..."
tail -n +2 "$UTXO_DUMP" > "$UTXO_DUMP.tmp" && mv "$UTXO_DUMP.tmp" "$UTXO_DUMP"

echo "Sorting the file..."

# Set the locale to make `sort -n` deterministic.
export LANG=C.UTF-8
export LANGUAGE=
export LC_CTYPE=C.UTF-8
export LC_NUMERIC="C.UTF-8"
export LC_TIME="C.UTF-8"
export LC_COLLATE="C.UTF-8"
export LC_MONETARY="C.UTF-8"
export LC_MESSAGES="C.UTF-8"
export LC_PAPER="C.UTF-8"
export LC_NAME="C.UTF-8"
export LC_ADDRESS="C.UTF-8"
export LC_TELEPHONE="C.UTF-8"
export LC_MEASUREMENT="C.UTF-8"
export LC_IDENTIFICATION="C.UTF-8"
export LC_ALL=

sort -n -o "$UTXO_DUMP" "$UTXO_DUMP"

echo "Computing sorted UTXO checksum..."
sha256sum "$UTXO_DUMP"
