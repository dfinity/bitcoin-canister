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
    get_utxos_base = 5000000000;
    get_utxos_cycles_per_ten_instructions = 500000000;
    get_utxos_maximum = 500000000000;
    get_balance = 5000000000;
    get_balance_maximum = 500000000000;
    get_current_fee_percentiles = 5000000000;
    get_current_fee_percentiles_maximum = 500000000000;
    send_transaction_base = 500000000000;
    send_transaction_per_byte = 2000000000;
  };
  api_access = variant { enabled }
})"

check_charging()
{
  METHOD=$1
  RECORD=$2
  EXPECTED=$3

  WALLET="$(dfx identity get-wallet)"
  BEFORE_SEND_TRANSACTION=$(dfx wallet balance --precise)

  # Send invalid transaction.
  set +e
  SEND_TX_OUTPUT=$(dfx canister  --wallet="${WALLET}" call --with-cycles 624000000000 bitcoin "${METHOD}" "${RECORD}" 2>&1);
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

#test bitcoin_send_transaction
TX_BYTES="blob \"12341234789789\""
METHOD="bitcoin_send_transaction"
RECORD="(record { network = variant { regtest }; transaction = ${TX_BYTES}})"
EXPECTED="Cannot decode transaction"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}"
#Do not forgot to test "Sending transaction bitcoin network must succeed, Rejection code"


#test bitcoin_get_balance
METHOD="bitcoin_get_balance"
RECORD="(record { address = \"Bad address\"; network = variant { regtest } })"
EXPECTED="MalformedAddress"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}"

RECORD="(record { address = \"bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8\"; network = variant { regtest }; min_confirmations = opt 10 })"
EXPECTED="MinConfirmationsTooLarge"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}"

#test bitcoin_get_utxos
METHOD="bitcoin_get_utxos"
RECORD="(record { address = \"Bad address\"; network = variant { regtest } })"
EXPECTED="MalformedAddress"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}"

RECORD="(record { address = \"bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8\"; network = variant { regtest }; filter = opt variant {min_confirmations = 10} })"
EXPECTED="MinConfirmationsTooLarge"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}"

BAD_PAGE="blob \"12341234789789\""
RECORD="(record { address = \"bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8\"; network = variant { regtest }; filter = opt variant {page = ${BAD_PAGE}} })"
EXPECTED="MalformedPage"
check_charging "${METHOD}" "${RECORD}" "${EXPECTED}"



#UnknownTipBlockHash

echo "SUCCESS"
