#!/usr/bin/env bash
set -Eexuo pipefail

# Run dfx stop if we run into errors.
trap "dfx stop" ERR

rm -rf .dfx
dfx start --background

# Deploy a mock of the management canister that returns fake blocks.
dfx deploy --no-wallet management-canister-mock

# Deploy the bitcoin canister, setting the blocks_source to be the mock above.
dfx deploy --no-wallet bitcoin --argument "(record {
  stability_threshold = 2;
  network = variant { regtest };
  blocks_source = principal \"$(dfx canister id management-canister-mock)\"
})"

# Wait a few seconds for the block to be ingested.
sleep 5

# Fetch the balance of an address we expect to have funds.
BALANCE=$(dfx canister call bitcoin get_balance '(record {
  address = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8"
})')

dfx stop

# Verify that the balance is 50 BTC.
if [[ $BALANCE = "(5_000_000_000 : nat64)" ]]; then
  echo "SUCCESS"
  exit 0
else
  echo "FAIL"
  exit 1
fi
