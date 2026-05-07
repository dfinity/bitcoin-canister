#!/usr/bin/env bash

# Configure dfx.json to use pre-built WASM from wasms/ when present (e.g. in CI).
# When wasms/ is not present (local dev), dfx.json is left unchanged and the build step runs.
use_prebuilt_bitcoin_wasm() {
  if [[ -f ../wasms/ic-btc-canister.wasm.gz ]]; then
    sed -i.bak 's|"wasm": "../target/wasm32-unknown-unknown/release/ic-btc-canister.wasm.gz"|"wasm": "../wasms/ic-btc-canister.wasm.gz"|' dfx.json
    sed -i.bak 's|"build": "../scripts/build-canister.sh ic-btc-canister"|"build": "true"|' dfx.json
  fi
}

# Waits until the main chain of the bitcoin canister has reached a certain height.
wait_until_main_chain_height () {
  HEIGHT=$1
  ATTEMPTS=$2

  BITCOIN_CANISTER_ID=$(dfx canister id bitcoin)

  while
    METRICS=$(curl "http://$BITCOIN_CANISTER_ID.raw.localhost:8000/metrics")
    ! [[ "$METRICS" == *"main_chain_height $HEIGHT"* ]]; do
      ((ATTEMPTS-=1))

      if [[ $ATTEMPTS -eq 0 ]]; then
	echo "TIMED OUT"
	exit 1
      fi

      sleep 1
  done
}

# Waits until the stable chain of the bitcoin canister has reached a certain height.
wait_until_stable_height () {
  HEIGHT=$1
  ATTEMPTS=$2

  BITCOIN_CANISTER_ID=$(dfx canister id bitcoin)

  while
    METRICS=$(curl "http://$BITCOIN_CANISTER_ID.raw.localhost:8000/metrics")
    ! [[ "$METRICS" == *"stable_height $HEIGHT"* ]]; do
      ((ATTEMPTS-=1))

      if [[ $ATTEMPTS -eq 0 ]]; then
	echo "TIMED OUT"
	exit 1
      fi

      sleep 1
  done
}

# Returns the number of UTXOs found in a response.
num_utxos () {
  UTXOS=$1
  # Count the occurrences of a substring of a UTXO.
  echo "$UTXOS" | grep -o " height = " | wc -l | xargs echo
}
