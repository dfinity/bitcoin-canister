#!/usr/bin/env bash
#
# A script for building the UTXO dump text file.
set -euo pipefail

# Generate the UTXO set.
~/go/bin/bitcoin-utxo-dump -db ./data/chainstate -o utxodump.csv -f "height,txid,vout,amount,type,address,script,coinbase,nsize"

echo "Removing the headers from the file..."
tail -n +2 utxodump.csv > utxodump.csv.tmp && mv utxodump.csv.tmp utxodump.csv

echo "Sorting the file..."
sort -n -o utxodump.csv utxodump.csv

echo "Computing sorted UTXO checksum..."
sha256sum utxodump.csv
