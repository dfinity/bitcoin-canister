#!/usr/bin/env bash
#
# Verify that the Bitcoin canister respects the `disable_api_if_not_fully_synced` flag.
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the canister that returns the blocks.
dfx deploy --no-wallet e2e-disable-api-if-not-fully-synced-flag

# Configure dfx.json to use pre-built WASM
use_prebuilt_bitcoin_wasm

# Deploy the bitcoin canister, setting the blocks_source to be the source above.
# And enabling 'disable_api_if_not_fully_synced'.
dfx deploy --no-wallet bitcoin --argument "(variant {init = record {
  stability_threshold = opt 1;
  network = opt variant { regtest };
  blocks_source = opt principal \"$(dfx canister id e2e-disable-api-if-not-fully-synced-flag)\";
  disable_api_if_not_fully_synced = opt variant { enabled };
}})"

# Wait until the ingestion of stable blocks is complete.
# The number of next block headers should be 3, the canister
# should reject all requests.
wait_until_stable_height 2 60

# bitcoin_get_balance should panic.
set +e
MSG=$(dfx canister call bitcoin bitcoin_get_balance '(record {
  network = variant { regtest };
  address = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8"
})' 2>&1);
set -e

if ! [[ $MSG = *"Canister state is not fully synced."* ]]; then
  echo "FAIL"
  exit 1
fi

# bitcoin_get_balance_query should panic.
set +e
MSG=$(dfx canister call --query bitcoin bitcoin_get_balance_query '(record {
  network = variant { regtest };
  address = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8"
})' 2>&1);
set -e

if ! [[ $MSG = *"Canister state is not fully synced."* ]]; then
  echo "FAIL"
  exit 1
fi

# bitcoin_get_utxos should panic.
set +e
MSG=$(dfx canister call bitcoin bitcoin_get_utxos '(record {
  network = variant { regtest };
  address = "bcrt1qxp8ercrmfxlu0s543najcj6fe6267j97tv7rgf";
})' 2>&1);
set -e

if ! [[ $MSG = *"Canister state is not fully synced."* ]]; then
  echo "FAIL"
  exit 1
fi

# bitcoin_get_block_headers should panic.
set +e
MSG=$(dfx canister call bitcoin bitcoin_get_block_headers '(record {
  start_height = 0;
  network = variant { regtest };
})' 2>&1);
set -e

if ! [[ $MSG = *"Canister state is not fully synced."* ]]; then
  echo "FAIL"
  exit 1
fi

# bitcoin_get_utxos_query should panic.
set +e
MSG=$(dfx canister call --query bitcoin bitcoin_get_utxos_query '(record {
  network = variant { regtest };
  address = "bcrt1qxp8ercrmfxlu0s543najcj6fe6267j97tv7rgf";
})' 2>&1);
set -e

if ! [[ $MSG = *"Canister state is not fully synced."* ]]; then
  echo "FAIL"
  exit 1
fi

# bitcoin_get_current_fee_percentiles should panic.
set +e
MSG=$(dfx canister call bitcoin bitcoin_get_current_fee_percentiles '(record {
  network = variant { regtest };
})' 2>&1);
set -e

if ! [[ $MSG = *"Canister state is not fully synced."* ]]; then
  echo "FAIL"
  exit 1
fi

dfx stop

dfx start --background --clean

# Deploy the canister that returns the blocks.
dfx deploy --no-wallet e2e-disable-api-if-not-fully-synced-flag

# Deploy the bitcoin canister, setting the blocks_source to be the source above.
# And disabling 'disable_api_if_not_fully_synced'. Hence, it should not make
# influence behaviour of the canister.
dfx deploy --no-wallet bitcoin --argument "(variant {init = record {
  stability_threshold = opt 1;
  network = opt variant { regtest };
  blocks_source = opt principal \"$(dfx canister id e2e-disable-api-if-not-fully-synced-flag)\";
  disable_api_if_not_fully_synced = opt variant { disabled };
}})"

# Wait until the ingestion of stable blocks is complete.
wait_until_main_chain_height 2 60

BALANCE=$(dfx canister call bitcoin bitcoin_get_balance '(record {
  network = variant { regtest };
  address = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8"
})')

if ! [[ $BALANCE = "(2 : nat64)" ]]; then
  echo "FAIL"
  exit 1
fi

# Verify that we are able to fetch the UTXOs of one address.
UTXOS=$(dfx canister call bitcoin bitcoin_get_utxos '(record {
  network = variant { regtest };
  address = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8"
})')

# The address has 2 UTXOs.
if ! [[ $(num_utxos "$UTXOS") = 2 ]]; then
  echo "FAIL"
  exit 1
fi

FEES=$(dfx canister call bitcoin bitcoin_get_current_fee_percentiles '(record {
  network = variant { regtest };
})')

if ! [[ $FEES = "(vec {})" ]]; then
  echo "FAIL"
  exit 1
fi

# Verify that we are able to fetch block headers.
MSG=$(dfx canister call bitcoin bitcoin_get_block_headers '(record {
  start_height = 0;
  network = variant { regtest };
})');

# Height of the tip is 2.
if ! [[ $MSG = *"tip_height = 2"* ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
