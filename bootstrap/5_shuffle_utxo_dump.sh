#!/usr/bin/env bash
#
# A script for shuffling the UTXO set. This step helps reduce the memory footprint of
# the stable btreemaps in the canister.
set -euo pipefail

echo "Shuffling the UTXO dump..."
awk 'BEGIN{srand(0);} {printf "%06d %s\n", rand()*1000000, $0;}' utxodump.csv | sort -n | cut -c8- > utxodump_shuffled.csv

echo "Computing shuffled UTXO checksum..."
sha256sum utxodump_shuffled.csv
