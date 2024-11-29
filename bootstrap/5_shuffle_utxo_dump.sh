#!/usr/bin/env bash
#
# A script for shuffling the UTXO set. This step helps reduce the memory footprint of
# the stable btreemaps in the canister.
set -euo pipefail

source "./utils.sh"

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

echo "Shuffling the UTXO dump..."
awk 'BEGIN{srand(0);} {printf "%06d %s\n", rand()*1000000, $0;}' "$UTXO_DUMP" | sort -n | cut -c8- > "$UTXO_DUMP_SHUFFLED"

echo "Computing shuffled UTXO checksum..."
sha256sum "$UTXO_DUMP_SHUFFLED"
