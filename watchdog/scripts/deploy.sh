#!/usr/bin/env bash
set -euo pipefail

# Hardcoded values.
BITCOIN_CANISTER_ID_MAINNET=ghsi2-tqaaa-aaaan-aaaca-cai
BITCOIN_CANISTER_ID_TESTNET=g4xu7-jiaaa-aaaan-aaaaq-cai
CANISTER_IDS_JSON=./canister_ids.json
WATCHDOG_CANISTER_ID_MAINNET=gatoo-6iaaa-aaaan-aaacq-cai
WATCHDOG_CANISTER_ID_TESTNET=gjqfs-iaaaa-aaaan-aaada-cai

# Verify that an argument was provided.
if [ $# -eq 0 ]; then
  echo "No arguments provided"
  echo "Usage: $0 [mainnet|testnet]"
  exit 1
fi

# Read network type from command line argument.
NETWORK_TYPE=$1

# Verify that network type is either mainnet or testnet.
if [ "$NETWORK_TYPE" != "mainnet" ] && [ "$NETWORK_TYPE" != "testnet" ]; then
  echo "Invalid network type: $NETWORK_TYPE"
  echo "Usage: $0 [mainnet|testnet]"
  exit 1
fi

# Populate variables depending on network type.
if [ "$NETWORK_TYPE" == "mainnet" ]; then
  BITCOIN_CANISTER_ID=$BITCOIN_CANISTER_ID_MAINNET
  WATCHDOG_CANISTER_ID=$WATCHDOG_CANISTER_ID_MAINNET
  EXPLORERS="vec {
    variant { api_blockchair_com_mainnet };
    variant { api_blockcypher_com_mainnet };
    variant { blockchain_info_mainnet };
    variant { blockstream_info_mainnet };
    variant { chain_api_btc_com_mainnet };
  }"
  # Below are disabled explorers (misbehaving, obsolete, etc).
  #  variant { api_bitaps_com_mainnet };
else
  BITCOIN_CANISTER_ID=$BITCOIN_CANISTER_ID_TESTNET
  WATCHDOG_CANISTER_ID=$WATCHDOG_CANISTER_ID_TESTNET
  EXPLORERS="vec {
    variant { api_bitaps_com_testnet };
    variant { api_blockchair_com_testnet };
    variant { api_blockcypher_com_testnet };
    variant { blockstream_info_testnet };
  }"
fi

# Run cargo tests, if they fail, exit.
cargo test -p watchdog

# Create canister_ids.json file.
echo "{
    \"watchdog\": {
        \"ic\": \"$WATCHDOG_CANISTER_ID\"
    }
}" > $CANISTER_IDS_JSON

# Deploy the canister.
dfx deploy --network ic watchdog --no-wallet --argument "(opt record {
    bitcoin_network = variant { $NETWORK_TYPE };
    blocks_behind_threshold = 2;
    blocks_ahead_threshold = 2;
    min_explorers = 2;
    bitcoin_canister_principal = principal \"${BITCOIN_CANISTER_ID}\";
    delay_before_first_fetch_sec = 1;
    interval_between_fetches_sec = 240;
    explorers = $EXPLORERS;
})"
