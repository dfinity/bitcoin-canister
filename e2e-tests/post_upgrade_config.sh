#!/usr/bin/env bash
#
# A test that verifies that calling post_upgrade with a set_config_request works.

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the bitcoin canister.
dfx deploy --no-wallet bitcoin --argument "(record {
  stability_threshold = opt 0;
  network = opt variant { regtest };
})"

# The stability threshold is zero
CONFIG=$(dfx canister call bitcoin get_config --query)
if ! [[ $CONFIG == *"stability_threshold = 0"* ]]; then
  echo "FAIL"
  exit 1
fi

# Modify the candid file such that post_upgrade can accept a `opt set_config_request`.
# This is necessary because post_upgrade takes a different argument from `init`, but candid
# doesn't provide a way of specifying that and dfx doesn't provide a way to bypass type checks.
sed -i.bak 's/service bitcoin : (init_config)/service bitcoin : (opt set_config_request)/' ./canister/candid.did

# Upgrade and update the fees.
FEES="record { get_current_fee_percentiles = 6 : nat; get_utxos_maximum = 3 : nat; get_block_headers_cycles_per_ten_instructions = 11 : nat; get_current_fee_percentiles_maximum = 7 : nat; send_transaction_per_byte = 9 : nat; get_balance = 4 : nat; get_utxos_cycles_per_ten_instructions = 2 : nat; get_block_headers_base = 10 : nat; get_utxos_base = 1 : nat; get_balance_maximum = 5 : nat; send_transaction_base = 8 : nat; get_block_headers_maximum = 12 : nat; }";

dfx deploy --upgrade-unchanged bitcoin --argument "opt (record {
  fees = opt $FEES;
})"

# Revert the modification to the candid file.
sed -i.bak 's/service bitcoin : (opt set_config_request)/service bitcoin : (init_config)/' ./canister/candid.did

# Verify the fees have been updated.
CONFIG=$(dfx canister call bitcoin get_config --query)
echo $CONFIG
if ! [[ $CONFIG == *"$FEES"* ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
