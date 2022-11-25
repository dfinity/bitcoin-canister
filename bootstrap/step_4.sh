#!/usr/bin/env bash
set -euo pipefail

# Generate the UTXO set.
~/go/bin/bitcoin-utxo-dump -db ./data/chainstate -o utxodump.csv -f "height,txid,vout,amount,type,address,script,coinbase,nsize"

echo "Removing the headers from the file..."
tail -n +2 utxodump.csv > utxodump.csv.tmp && mv utxodump.csv.tmp utxodump.csv

echo "Sorting the file..."
sort -n -o utxodump.csv utxodump.csv

echo "Computing sorted UTXO checksum..."
sha256sum utxodump.csv

echo "Shuffling the file..."
# Shuffling helps reduce the memory footprint of the stable btreemaps in the canister.
awk 'BEGIN{srand(0);} {printf "%06d %s\n", rand()*1000000, $0;}' utxodump.csv | sort -n | cut -c8- > utxodump.csv.tmp && mv utxodump.csv.tmp utxodump.csv

echo "Computing shuffled UTXO checksum..."
sha256sum utxodump.csv
