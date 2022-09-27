#!/usr/bin/env bash
set -Eexuo pipefail

# Run dfx stop if we run into errors.
trap "dfx stop" ERR EXIT

# Waits until the main chain of the bitcoin canister has reached a certain height.
wait_until_height () {
  HEIGHT=$1
  ATTEMPTS=$2

  BITCOIN_CANISTER_ID=$(dfx canister id bitcoin)

  while
    METRICS=$(curl "http://127.0.0.1:8000/metrics?canisterId=$BITCOIN_CANISTER_ID")
    ! [[ "$METRICS" == *"main_chain_height $HEIGHT"* ]]; do
      ((ATTEMPTS-=1))

      if [[ $ATTEMPTS -eq 0 ]]; then
	echo "TIMED OUT"
	exit 1
      fi

      sleep 1
  done
}

rm -rf .dfx
dfx start --background

# Deploy a mock of the management canister that returns fake blocks.
dfx deploy --no-wallet management-canister-mock

# Deploy the bitcoin canister, setting the blocks_source to be the mock above.
dfx deploy --no-wallet bitcoin --argument "(record {
  stability_threshold = 1;
  network = variant { regtest };
  blocks_source = principal \"$(dfx canister id management-canister-mock)\"
})"

# Wait until the chain is at height 3 (and for at most 10 seconds).
wait_until_height 3 10

# Fetch the balance of an address we expect to have funds.
BALANCE=$(dfx canister call bitcoin get_balance '(record {
  address = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8"
})')

# Verify that the balance is 50 BTC.
if ! [[ $BALANCE = "(5_000_000_000 : nat64)" ]]; then
  echo "FAIL"
  exit 1
fi

# Fetch the balance of an address we expect to have funds.
BALANCE=$(dfx canister call bitcoin get_balance '(record {
  address = "bcrt1qxp8ercrmfxlu0s543najcj6fe6267j97tv7rgf"
})')

# Verify that the balance is 50 BTC.
if [[ $BALANCE = "(5_000_000_000 : nat64)" ]]; then
  echo "SUCCESS"
  exit 0
else
  echo "FAIL"
  exit 1
fi
