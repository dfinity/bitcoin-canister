#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the canister that returns the blocks for scenario 1.
dfx deploy --no-wallet e2e-scenario-1

# Deploy the bitcoin canister, setting the blocks_source to be the source above.
dfx deploy --no-wallet bitcoin --argument "(record {
  stability_threshold = 2;
  network = variant { regtest };
  blocks_source = principal \"$(dfx canister id e2e-scenario-1)\";
  syncing = variant { enabled };
  fees = record {
    get_utxos_base = 0;
    get_utxos_cycles_per_ten_instructions = 0;
    get_utxos_maximum = 0;
    get_balance = 0;
    get_balance_maximum = 0;
    get_current_fee_percentiles = 0;
    get_current_fee_percentiles_maximum = 0;
    send_transaction_base = 0;
    send_transaction_per_byte = 0;
  };
  api_access = variant { enabled }
})"

# Wait until the ingestion of stable blocks is complete.
wait_until_stable_height 3 60

# Fetch the balance of an address we do not expect to have funds.
BALANCE=$(dfx canister call bitcoin bitcoin_get_balance '(record {
  network = variant { regtest };
  address = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8"
})')

if ! [[ $BALANCE = "(0 : nat64)" ]]; then
  echo "FAIL"
  exit 1
fi

# Fetch the balance of an address we expect to have funds.
BALANCE=$(dfx canister call bitcoin bitcoin_get_balance '(record {
  network = variant { regtest };
  address = "bcrt1qxp8ercrmfxlu0s543najcj6fe6267j97tv7rgf";
  min_confirmations = opt 2;
})')

# Verify that the balance is 50 BTC.
if ! [[ $BALANCE = "(5_000_000_000 : nat64)" ]]; then
  echo "FAIL"
  exit 1
fi

UTXOS=$(dfx canister call bitcoin bitcoin_get_utxos '(record {
  network = variant { regtest };
  address = "bcrt1qxp8ercrmfxlu0s543najcj6fe6267j97tv7rgf";
})')

# The address has no UTXOs.
if ! [[ $(num_utxos "$UTXOS") = 0 ]]; then
  echo "FAIL"
  exit 1
fi

# Verify that we are able to fetch the UTXOs of one address.
# We temporarily pause outputting the commands to the terminal as
# this command would print thousands of UTXOs.
set +x
UTXOS=$(dfx canister call bitcoin bitcoin_get_utxos '(record {
  network = variant { regtest };
  address = "bcrt1qenhfslne5vdqld0djs0h0tfw225tkkzzc60exh"
})')

# The address has 10000 UTXOs, but the response is capped to 1000 UTXOs.
if ! [[ $(num_utxos "$UTXOS") = 1000 ]]; then
  echo "FAIL"
  exit 1
fi
set -x

BALANCE=$(dfx canister call bitcoin bitcoin_get_balance '(record {
  network = variant { regtest };
  address = "bcrt1qenhfslne5vdqld0djs0h0tfw225tkkzzc60exh";
})')

if ! [[ $BALANCE = "(5_000_000_000 : nat64)" ]]; then
  echo "FAIL"
  exit 1
fi

# Request the current fee percentiles. This is only for profiling purposes.
dfx canister call bitcoin bitcoin_get_current_fee_percentiles '(record {
  network = variant { regtest };
})'
dfx canister call bitcoin bitcoin_get_current_fee_percentiles '(record {
  network = variant { regtest };
})'

echo "SUCCESS"
