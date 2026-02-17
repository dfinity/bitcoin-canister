#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
pushd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the canister that returns the blocks for scenario 1.
dfx deploy --no-wallet e2e-scenario-1

# Configure dfx.json to use pre-built WASM
use_prebuilt_bitcoin_wasm

# Deploy the bitcoin canister, setting the blocks_source to be the source above.
dfx deploy --no-wallet bitcoin --argument "(variant {init = record {
  stability_threshold = opt 2;
  network = opt variant { regtest };
  blocks_source = opt principal \"$(dfx canister id e2e-scenario-1)\";
}})"

# Wait until all blocks have been received.
wait_until_main_chain_height 5 60

# Verify the blockchain info using the query endpoint.
BLOCKCHAIN_INFO=$(dfx canister call bitcoin get_blockchain_info --query)
if ! [[ $BLOCKCHAIN_INFO == *"height = 5"* ]]; then
  echo "FAIL: Expected height 5 in blockchain info, got $BLOCKCHAIN_INFO"
  exit 1
fi

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

BALANCE=$(dfx canister call --query bitcoin bitcoin_get_balance_query '(record {
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

UTXOS=$(dfx canister call --query bitcoin bitcoin_get_utxos_query '(record {
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
UTXOS=$(dfx canister call --query bitcoin bitcoin_get_utxos_query '(record {
  network = variant { regtest };
  address = "bcrt1qenhfslne5vdqld0djs0h0tfw225tkkzzc60exh"
})')

# The address has 10000 UTXOs, but the response is capped to 1000 UTXOs.
if ! [[ $(num_utxos "$UTXOS") = 1000 ]]; then
  echo "FAIL"
  exit 1
fi
set -x

set +x
UTXOS=$(dfx canister call bitcoin bitcoin_get_utxos_query '(record {
  network = variant { regtest };
  address = "bcrt1qenhfslne5vdqld0djs0h0tfw225tkkzzc60exh"
})')

# The address has 10000 UTXOs, but the response is capped to 1000 UTXOs.
if ! [[ $(num_utxos "$UTXOS") = 1000 ]]; then
  echo "FAIL"
  exit 1
fi
set -x

# Check that 'bitcoin_get_utxos_query' cannot be called in replicated mode.
set +e
GET_UTXOS_QUERY_REPLICATED_CALL=$(dfx canister call --update bitcoin bitcoin_get_utxos_query '(record {
  network = variant { regtest };
  address = "bcrt1qenhfslne5vdqld0djs0h0tfw225tkkzzc60exh";
})' 2>&1)
set -e

if [[ $GET_UTXOS_QUERY_REPLICATED_CALL != *"CanisterReject"* ]]; then
  echo "FAIL"
  exit 1
fi

BALANCE=$(dfx canister call --query bitcoin bitcoin_get_balance_query '(record {
  network = variant { regtest };
  address = "bcrt1qenhfslne5vdqld0djs0h0tfw225tkkzzc60exh";
})')

if ! [[ $BALANCE = "(5_000_000_000 : nat64)" ]]; then
  echo "FAIL"
  exit 1
fi

# Check that 'bitcoin_get_balance_query' cannot be called in replicated mode.
set +e
GET_BALANCE_QUERY_REPLICATED_CALL=$(dfx canister call --update bitcoin bitcoin_get_balance_query '(record {
  network = variant { regtest };
  address = "bcrt1qenhfslne5vdqld0djs0h0tfw225tkkzzc60exh";
})' 2>&1)
set -e

if [[ $GET_BALANCE_QUERY_REPLICATED_CALL != *"CanisterReject"* ]]; then
  echo "FAIL"
  exit 1
fi

BALANCE=$(dfx canister call bitcoin bitcoin_get_balance '(record {
  network = variant { regtest };
  address = "bcrt1qenhfslne5vdqld0djs0h0tfw225tkkzzc60exh";
})')

if ! [[ $BALANCE = "(5_000_000_000 : nat64)" ]]; then
  echo "FAIL"
  exit 1
fi

BALANCE=$(dfx canister call --query bitcoin bitcoin_get_balance_query '(record {
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

# Verify that we can fetch the block headers.
ACTUAL_HEADERS=$(dfx canister call bitcoin bitcoin_get_block_headers '(record {
  start_height = 0;
  network = variant { regtest };
})');

# The e2e-scenario-1 canister chains 5 blocks onto the genesis block.
EXPECTED_HEADERS='(
  record {
    tip_height = 5 : nat32;
    block_headers = vec {
      blob "\01\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\3b\a3\ed\fd\7a\7b\12\b2\7a\c7\2c\3e\67\76\8f\61\7f\c8\1b\c3\88\8a\51\32\3a\9f\b8\aa\4b\1e\5e\4a\da\e5\49\4d\ff\ff\7f\20\02\00\00\00";
      blob "\01\00\00\00\06\22\6e\46\11\1a\0b\59\ca\af\12\60\43\eb\5b\bf\28\c3\4f\3a\5e\33\2a\1f\c7\b2\b7\3c\f1\88\91\0f\f0\bd\3e\7d\a3\bc\8d\c6\62\68\28\b3\66\7a\16\ba\4e\ef\63\96\6a\68\eb\4d\fd\ae\d7\f1\6f\41\97\c8\32\e8\49\4d\ff\ff\7f\20\00\00\00\00";
      blob "\01\00\00\00\b5\2a\48\82\73\2c\0c\e4\6f\9c\91\a3\71\e3\ee\7f\33\02\9b\09\50\2d\af\59\8e\5e\2d\4e\c2\00\89\56\f2\83\4a\e9\a7\78\d3\58\67\63\7e\17\b9\f6\75\5e\03\dd\bb\8c\52\1b\9a\d6\07\b5\bb\ab\ee\a1\15\33\8a\ea\49\4d\ff\ff\7f\20\00\00\00\00";
      blob "\01\00\00\00\9d\9d\5d\b6\5e\61\2a\f4\ef\18\e2\50\a8\2a\30\8e\a1\d3\49\eb\96\88\3b\12\1c\90\52\35\6d\83\10\69\7e\de\e2\2e\85\73\88\87\ce\80\9e\c6\cf\df\6c\ba\43\cc\ee\51\a9\6e\9a\e6\ba\e9\22\71\39\c5\e2\07\e2\ec\49\4d\ff\ff\7f\20\01\00\00\00";
      blob "\01\00\00\00\c2\34\c0\c4\59\61\6d\2c\1f\b0\ab\a3\92\f5\e7\c2\5d\e3\83\3b\9b\35\a7\41\1c\4e\9d\08\15\27\fd\55\47\e2\c5\8e\39\9b\85\d6\fc\e6\bc\46\7d\52\1a\5a\6f\54\1f\02\4c\e2\8e\88\27\cd\e1\e4\23\b2\13\3a\3a\ef\49\4d\ff\ff\7f\20\02\00\00\00";
      blob "\01\00\00\00\09\ca\ab\ac\0a\f4\33\86\14\54\63\62\3f\e9\15\03\2e\ec\a0\da\02\1b\03\a0\48\be\22\21\fc\d7\49\54\00\51\6d\88\c9\36\80\03\be\61\36\ce\35\41\8b\d3\ac\40\9f\1c\ab\5c\ed\ac\4e\bb\56\33\34\9b\fa\e5\92\f1\49\4d\ff\ff\7f\20\01\00\00\00";
    };
  },
)'

if ! [[ $ACTUAL_HEADERS = "$EXPECTED_HEADERS" ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
