#!/usr/bin/env bash
set -Eexuo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"
cd "$SCRIPT_DIR"

# Run dfx stop if we run into errors.
trap "dfx stop" EXIT SIGINT

dfx start --background --clean

# Deploy the canister that returns the blocks for scenario 1.
dfx deploy --no-wallet e2e-scenario-1

# Deploy the bitcoin canister, setting the blocks_source to be the source above.
dfx deploy --no-wallet bitcoin --argument "(record {
  stability_threshold = opt 2;
  network = opt variant { regtest };
  blocks_source = opt principal \"$(dfx canister id e2e-scenario-1)\";
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
      blob "\01\00\00\00\06\22\6e\46\11\1a\0b\59\ca\af\12\60\43\eb\5b\bf\28\c3\4f\3a\5e\33\2a\1f\c7\b2\b7\3c\f1\88\91\0f\8b\c1\34\a4\62\25\ea\ec\84\54\1e\8c\0f\61\25\5d\cb\d4\16\b4\82\fd\17\94\c1\f3\24\90\30\1e\77\c3\32\e8\49\4d\ff\ff\7f\20\01\00\00\00";
      blob "\01\00\00\00\6c\e6\6b\7d\21\59\86\39\89\71\41\5f\a4\67\97\ff\52\e8\cf\17\65\f8\e7\ab\53\65\3d\a6\3e\97\18\1c\bf\8d\e3\75\cc\c0\3d\ba\b1\80\da\34\00\7f\0e\bb\ea\a1\30\29\48\dc\07\5b\7a\92\81\d0\de\01\6a\a6\8a\ea\49\4d\ff\ff\7f\20\01\00\00\00";
      blob "\01\00\00\00\55\07\55\91\fd\ec\e1\02\ab\1f\95\d8\62\ea\98\07\00\c6\c1\61\33\bd\fa\06\86\e8\11\8e\a0\77\76\2d\45\29\a5\be\2b\cc\e3\ea\57\72\a2\d0\b6\69\f9\34\a2\d0\9e\18\c7\91\72\50\52\2e\ac\b2\a4\49\ac\b0\e2\ec\49\4d\ff\ff\7f\20\00\00\00\00";
      blob "\01\00\00\00\be\62\5e\24\a2\e7\38\ec\96\b4\1b\94\ff\1f\8f\ce\7f\f1\50\76\9c\78\74\fc\2d\ea\97\11\d4\ff\85\1d\ac\4d\af\1b\59\7f\e6\c8\18\0d\28\ee\93\c9\c9\aa\bc\4e\99\30\eb\5d\ad\00\a5\aa\3f\22\79\b8\83\92\3a\ef\49\4d\ff\ff\7f\20\01\00\00\00";
      blob "\01\00\00\00\5e\57\ba\9c\a3\3a\f4\b7\99\9d\ea\0f\9f\a1\5d\c7\12\cb\54\d6\3d\ed\c8\8d\35\7a\d1\c2\13\1e\08\18\4a\a8\63\98\0d\83\85\8f\00\6f\3f\1f\0d\4e\ca\67\7e\15\c0\c2\2d\e1\ae\2a\eb\83\e3\0b\7d\10\23\3a\92\f1\49\4d\ff\ff\7f\20\01\00\00\00";
    };
  },
)'

if ! [[ $ACTUAL_HEADERS = "$EXPECTED_HEADERS" ]]; then
  echo "FAIL"
  exit 1
fi

echo "SUCCESS"
