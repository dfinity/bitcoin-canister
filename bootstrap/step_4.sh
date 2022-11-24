#!/usr/bin/env bash
set -euo pipefail

CANISTER_STATE_DIR=canister_state
UTXO_FILE=utxodump.csv

mkdir $CANISTER_STATE_DIR

echo "Computing balances..."
cargo run --release --bin build-balances -- \
   --output $CANISTER_STATE_DIR/balances --network mainnet --utxos-dump-path $UTXO_FILE

echo "Computing address UTXOs..."
cargo run --release --bin build-address-utxos -- \
   --output $CANISTER_STATE_DIR/address_utxos --network mainnet --utxos-dump-path $UTXO_FILE

echo "Computing UTXOs..."
cargo run --release --bin build-utxos -- \
   --output $CANISTER_STATE_DIR --network mainnet --utxos-dump-path $UTXO_FILE
