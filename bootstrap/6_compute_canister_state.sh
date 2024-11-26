#!/usr/bin/env bash
#
# A script to build the canister's state given a UTXO dump file.
set -euo pipefail

source "$(dirname "$0")/utils.sh"

CANISTER_STATE_DIR=canister_state
CANISTER_STATE_FILE=canister_state.bin
UTXO_FILE=utxodump_shuffled.csv
UNSTABLE_BLOCKS_FILE=unstable_blocks
BLOCK_HEADERS_FILE=block_headers

NETWORK=$1
HEIGHT=$2
STABILITY_THRESHOLD=$3

ANCHOR_HEIGHT=$((HEIGHT-11))

validate_network "$NETWORK"

mkdir $CANISTER_STATE_DIR

echo "Computing balances..."
cargo run -p state-builder --release --bin build-balances -- \
   --output $CANISTER_STATE_DIR/balances --network "$NETWORK" --utxos-dump-path $UTXO_FILE

echo "Computing address UTXOs..."
cargo run -p state-builder --release --bin build-address-utxos -- \
   --output $CANISTER_STATE_DIR/address_utxos --network "$NETWORK" --utxos-dump-path $UTXO_FILE

echo "Computing UTXOs..."
cargo run -p state-builder --release --bin build-utxos --features file_memory -- \
   --output $CANISTER_STATE_DIR --network "$NETWORK" --utxos-dump-path $UTXO_FILE

echo "Combining the state into $CANISTER_STATE_FILE"
cargo run -p state-builder --release --bin combine-state -- \
   --output $CANISTER_STATE_FILE --canister-state-dir $CANISTER_STATE_DIR

echo "Building state struct.."
cargo run -p state-builder --release --bin main-state-builder --features file_memory -- \
   --canister-state "$CANISTER_STATE_FILE" \
   --canister-state-dir "$CANISTER_STATE_DIR" \
   --network "$NETWORK" \
   --stability-threshold "$STABILITY_THRESHOLD" \
   --anchor-height "$ANCHOR_HEIGHT" \
   --unstable-blocks "$UNSTABLE_BLOCKS_FILE" \
   --block-headers "$BLOCK_HEADERS_FILE"

echo "Computing checksum of canister state..."
sha256sum "$CANISTER_STATE_FILE"
