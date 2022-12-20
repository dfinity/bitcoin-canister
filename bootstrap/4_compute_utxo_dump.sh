#!/usr/bin/env bash
#
# A script for building the UTXO dump text file.
set -euo pipefail

NETWORK=$1

if ! [[ "$NETWORK" == "mainnet" || "$NETWORK" == "testnet" ]]; then
    echo "NETWORK must be set to either 'mainnet' or 'testnet'"
    false
fi

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
sort -n -o utxodump.csv utxodump.csv

echo "Computing sorted UTXO checksum..."
sha256sum utxodump.csv
