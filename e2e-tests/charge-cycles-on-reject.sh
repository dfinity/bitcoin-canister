#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the canister that returns the blocks for harge-cycles-on-reject.
dfx deploy e2e-scenario-3

# Deploy the bitcoin canister, setting the blocks_source to be the source above.
dfx deploy bitcoin --argument "(record {
  stability_threshold = 2;
  network = variant { regtest };
  blocks_source = principal \"$(dfx canister id e2e-scenario-3)\";
  syncing = variant { enabled };
  fees = record {
    get_utxos_base = 500000000000;
    get_utxos_cycles_per_ten_instructions = 500000000000;
    get_utxos_maximum = 500000000000;
    get_balance = 500000000000;
    get_balance_maximum = 500000000000;
    get_current_fee_percentiles = 500000000000;
    get_current_fee_percentiles_maximum = 500000000000;
    send_transaction_base = 500000000000;
    send_transaction_per_byte = 2000000000;
  };
  api_access = variant { enabled }
})"

check_charging()
{
  WALLET=$1
  METHOD=$2
  RECORD=$3
  EXPECTED=$4

  BEFORE_SEND_TRANSACTION=$(dfx wallet balance --precise)

  # Send invalid transaction.
  set +e
  SEND_TX_OUTPUT=$(dfx canister  --wallet=${WALLET} call --with-cycles 624000000000 bitcoin ${METHOD} "${RECORD}" 2>&1);
  set -e


  # Should reject.
  if [[ $SEND_TX_OUTPUT != *"${EXPECTED}"* ]]; then
    echo "FAIL"
    exit 1
  fi

  AFTER_SEND_TRANSACTION=$(dfx wallet balance --precise)

  # Should charge cycles.
  if [[ $BEFORE_SEND_TRANSACTION = "$AFTER_SEND_TRANSACTION" ]]; then
    echo "FAIL"
    exit 1
  fi
}

WALLET="$(dfx identity get-wallet)"
TX_BYTES="blob \"12341234789789\""
METHOD="bitcoin_send_transaction"
RECORD="(record { network = variant { regtest }; transaction = ${TX_BYTES}})"
EXPECTED="Cannot decode transaction"
check_charging "${WALLET}" "${METHOD}" "${RECORD}" "${EXPECTED}"

METHOD="bitcoin_get_balance"
RECORD="(record { address = \"Bad address\"; network = variant { regtest } })"
EXPECTED="MalformedAddress"
check_charging "${WALLET}" "${METHOD}" "${RECORD}" "${EXPECTED}"

METHOD="bitcoin_get_balance"
RECORD="(record { address = \"bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8\"; network = variant { regtest }; min_confirmations = opt 10 })"
EXPECTED="MinConfirmationsTooLarge"
check_charging "${WALLET}" "${METHOD}" "${RECORD}" "${EXPECTED}"

METHOD="bitcoin_get_utxos"
RECORD="(record { address = \"Bad address\"; network = variant { regtest } })"
EXPECTED="MalformedAddress"
check_charging "${WALLET}" "${METHOD}" "${RECORD}" "${EXPECTED}"

METHOD="bitcoin_get_utxos"
RECORD="(record { address = \"bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8\"; network = variant { regtest }; filter = opt variant {min_confirmations = 10} })"
EXPECTED="MinConfirmationsTooLarge"
check_charging "${WALLET}" "${METHOD}" "${RECORD}" "${EXPECTED}"

#METHOD="bitcoin_get_utxos"
#RECORD="(record { address = \"bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8\"; network = variant { regtest }; filter = opt variant {page = ${TX_BYTES}} })"
#EXPECTED="MalformedPage"
#check_charging "${WALLET}" "${METHOD}" "${RECORD}" "${EXPECTED}"

echo "SUCCESS"