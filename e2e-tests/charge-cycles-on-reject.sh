#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the canister that will be used as an internal endpoint to send the transaction 
# to the Bitcoin network when calling bitcoin_send_transaction.
dfx deploy e2e-scenario-1

# Configure dfx.json to use pre-built WASM
# (dfx deploy sets up wallet infrastructure needed for cycle charging tests)
use_prebuilt_bitcoin_wasm

# Deploy the bitcoin canister.
dfx deploy bitcoin --argument "(variant {init = record {
  stability_threshold = opt 2;
  network = opt variant { regtest };
  blocks_source = opt principal \"$(dfx canister id e2e-scenario-1)\";
  syncing = opt variant { enabled };
  fees = opt record {
    get_utxos_base = 1;
    get_utxos_cycles_per_ten_instructions = 1;
    get_utxos_maximum = 1;
    get_balance = 1;
    get_balance_maximum = 1;
    get_current_fee_percentiles = 1;
    get_current_fee_percentiles_maximum = 1;
    send_transaction_base = 1;
    send_transaction_per_byte = 1;
    get_block_headers_base = 1;
    get_block_headers_cycles_per_ten_instructions = 1;
    get_block_headers_maximum = 1;
  };
}})"

check_charging()
{
  METHOD=$1
  RECORD=$2
  EXPECTED_OUTPUT=$3
  EXPECTED_FEE=$4

  WALLET="$(dfx identity get-wallet)"
  BEFORE_SEND_TRANSACTION=$(dfx wallet balance --precise | tr -d -c 0-9)

  # Send invalid transaction.
  set +e
  SEND_TX_OUTPUT=$(dfx canister call --wallet "${WALLET}" --with-cycles "${EXPECTED_FEE}" bitcoin "${METHOD}" "${RECORD}" 2>&1);
  set -e


  # Should reject.
  if [[ $SEND_TX_OUTPUT != *"${EXPECTED_OUTPUT}"* ]]; then
    echo "FAIL"
    exit 1
  fi

  AFTER_SEND_TRANSACTION=$(dfx wallet balance --precise | tr -d -c 0-9)

  FEE=$(("${BEFORE_SEND_TRANSACTION}" - "${AFTER_SEND_TRANSACTION}"))

  # Should charge EXPECTED_FEE cycles.
  if [[ $FEE != "$EXPECTED_FEE" ]]; then
    echo "FAIL"
    exit 1
  fi
}

wait_until_main_chain_height 1 60

#test bitcoin_send_transaction
TX_BYTES="blob \"12341234789789\""
METHOD="bitcoin_send_transaction"
RECORD="(record { network = variant { regtest }; transaction = ${TX_BYTES}})"
EXPECTED="MalformedTransaction"
# Expected fee is 15 = 1 * send_transaction_base + 14 * send_transaction_per_byte
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}" 15

#test bitcoin_get_balance
METHOD="bitcoin_get_balance"
RECORD="(record { address = \"Bad address\"; network = variant { regtest } })"
EXPECTED="MalformedAddress"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}" 1

RECORD="(record { address = \"bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8\"; network = variant { regtest }; min_confirmations = opt 10 })"
EXPECTED="MinConfirmationsTooLarge"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}" 1

#test bitcoin_get_utxos
METHOD="bitcoin_get_utxos"
RECORD="(record { address = \"Bad address\"; network = variant { regtest } })"
EXPECTED="MalformedAddress"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}" 1

RECORD="(record { address = \"bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8\"; network = variant { regtest }; filter = opt variant {min_confirmations = 10} })"
EXPECTED="MinConfirmationsTooLarge"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}" 1

SHORT_PAGE="blob \"12341234789789\""
RECORD="(record { address = \"bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8\"; network = variant { regtest }; filter = opt variant {page = ${SHORT_PAGE}} })"
EXPECTED="MalformedPage"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}" 1

BAD_TIP="blob \"123412347897123412347897123412347897123412347897123412347897123412347897\""
RECORD="(record { address = \"bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8\"; network = variant { regtest }; filter = opt variant {page = ${BAD_TIP}} })"
EXPECTED="UnknownTipBlockHash"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}" 1

#test bitcoin_get_block_headers
METHOD="bitcoin_get_block_headers"
RECORD="(record { start_height = 10; network = variant { regtest }})"
EXPECTED="StartHeightDoesNotExist"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}" 1

METHOD="bitcoin_get_block_headers"
RECORD="(record { start_height = 0; end_height = opt 10; network = variant { regtest } })"
EXPECTED="EndHeightDoesNotExist"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}" 1

METHOD="bitcoin_get_block_headers"
RECORD="(record { start_height = 1; end_height = opt 0; network = variant { regtest } })"
EXPECTED="StartHeightLargerThanEndHeight"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}" 1

echo "SUCCESS"
