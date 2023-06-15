#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the canister that will be used as an internal endpoint to send the transaction 
# to the Bitcoin network when calling bitcoin_send_transaction.
dfx deploy e2e-scenario-3

# Deploy the bitcoin canister.
dfx deploy bitcoin --argument "(record {
  stability_threshold = 2;
  network = variant { regtest };
  blocks_source = principal \"$(dfx canister id e2e-scenario-3)\";
  syncing = variant { disabled };
  fees = record {
    get_utxos_base = 1;
    get_utxos_cycles_per_ten_instructions = 1;
    get_utxos_maximum = 1;
    get_balance = 1;
    get_balance_maximum = 1;
    get_current_fee_percentiles = 1;
    get_current_fee_percentiles_maximum = 1;
    send_transaction_base = 1;
    send_transaction_per_byte = 1;
  };
  api_access = variant { enabled };
  disable_api_if_not_fully_synced = variant { enabled }
  watchdog_canister = null;
})"

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
  SEND_TX_OUTPUT=$(dfx canister  --wallet="${WALLET}" call --with-cycles "${EXPECTED_FEE}" bitcoin "${METHOD}" "${RECORD}" 2>&1);
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

echo "SUCCESS"
