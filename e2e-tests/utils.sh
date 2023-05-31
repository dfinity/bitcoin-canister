#!/usr/bin/env bash

# Waits until the main chain of the bitcoin canister has reached a certain height.
wait_until_main_chain_height () {
  HEIGHT=$1
  ATTEMPTS=$2

  BITCOIN_CANISTER_ID=$(dfx canister id bitcoin)
  WEBSERVER_PORT=$(dfx info webserver-port)

  while
    METRICS=$(curl "http://127.0.0.1:$WEBSERVER_PORT/metrics?canisterId=$BITCOIN_CANISTER_ID")
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
  WEBSERVER_PORT=$(dfx info webserver-port)

  while
    METRICS=$(curl "http://127.0.0.1:$WEBSERVER_PORT/metrics?canisterId=$BITCOIN_CANISTER_ID")
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
