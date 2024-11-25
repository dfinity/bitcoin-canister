#!/usr/bin/env bash
#
# A script for building the UTXO dump text file.
set -euo pipefail

source "$(dirname "$0")/utils.sh"

NETWORK=$1

validate_network "$NETWORK"

# Generate the UTXO set.
if [[ "$NETWORK" == "mainnet" ]]; then
    CHAIN_STATE_DIR=./data/chainstate
else
    CHAIN_STATE_DIR=./data/testnet3/chainstate
fi

~/go/bin/bitcoin-utxo-dump -db "$CHAIN_STATE_DIR" -o utxodump.csv -f "height,txid,vout,amount,type,address,script,coinbase,nsize"

echo "Removing the headers from the file..."
tail -n +2 utxodump.csv > utxodump.csv.tmp && mv utxodump.csv.tmp utxodump.csv

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

sort -n -o utxodump.csv utxodump.csv

echo "Computing sorted UTXO checksum..."
sha256sum utxodump.csv
