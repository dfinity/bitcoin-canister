#!/usr/bin/env bash

# Function to deploy the watchdog canister for mainnet bitcoin_canister.
deploy_watchdog_canister_mainnet() {
  BITCOIN_NETWORK=mainnet
  BITCOIN_CANISTER_ID=ghsi2-tqaaa-aaaan-aaaca-cai
  dfx deploy --no-wallet watchdog --argument "(record {
    bitcoin_network = variant { ${BITCOIN_NETWORK} };
    blocks_behind_threshold = 2;
    blocks_ahead_threshold = 2;
    min_explorers = 2;
    bitcoin_canister_principal = principal \"${BITCOIN_CANISTER_ID}\";
    delay_before_first_fetch_sec = 1;
    interval_between_fetches_sec = 60;
  })"
}
