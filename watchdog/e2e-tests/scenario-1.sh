#!/usr/bin/env bash
set -Eexuo pipefail

# FAKE_EXPLORERS_ADDRESS="[::1]:8080"
FAKE_EXPLORERS_ADDRESS="127.0.0.1:8080"
DFX_HOST="127.0.0.1:8123"

# Source the utility functions.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
source "${SCRIPT_DIR}/utils.sh"

# Run dfx stop and error function if we run into errors.
trap error EXIT SIGINT

CERT_FILE="${SCRIPT_DIR}/fake-explorers/src/cert.crt"

# Start fake explorers.
# TODO: run fake explorers in the background.
cargo build --manifest-path "${SCRIPT_DIR}/fake-explorers/Cargo.toml"
cargo run --manifest-path "${SCRIPT_DIR}/fake-explorers/Cargo.toml" -- \
  --addr=$FAKE_EXPLORERS_ADDRESS \
  --cert="${CERT_FILE}" \
  --key="${SCRIPT_DIR}/fake-explorers/src/key.pem" \
  &
FAKE_EXPLORERS_PID=$!

# Maximum number of attempts to check the status.
max_attempts=5
count=1

# Wait for fake explorers to start up.
until curl -k -s "https://${FAKE_EXPLORERS_ADDRESS}/status" > /dev/null || [[ "$count" -eq "$max_attempts" ]]
do
  sleep 1
  count=$((count + 1))
  echo "Waiting for fake explorers to start... attempt: $count"
done

# Check if the maximum attempts was reached
if [[ "$count" -eq "$max_attempts" ]]; then
  echo "Failed to start fake explorers after $max_attempts attempts. Exiting."
  exit 1
fi

# EXPLORER=$(curl -k "https://${FAKE_EXPLORERS_ADDRESS}/api.bitaps.com/btc/v1/blockchain/block/last")
# echo $EXPLORER

# echo "Fake explorers PID: ${FAKE_EXPLORERS_PID}"

# Additional cleanup trap to kill the fake explorer process.
trap "kill -9 ${FAKE_EXPLORERS_PID}; $(trap -p EXIT | cut -d ' ' -f3-)" EXIT


# Start the local dfx.
echo "The SSL_CERT_FILE value is $CERT_FILE"
SSL_CERT_FILE=$CERT_FILE dfx start --background --clean --host $DFX_HOST

# Deploy fake explorers canister.
# dfx deploy --no-wallet watchdog-e2e-fake-explorers-canister
# EXPLORERS_CANISTER_ID=$(dfx canister id watchdog-e2e-fake-explorers-canister)
# EXPLORER=$(curl "https://127.0.0.1:8080/api.bitaps.com/btc/v1/blockchain/block/last?canisterId=$EXPLORERS_CANISTER_ID")
# echo $EXPLORER

# Deploy fake bitcoin canister.
dfx deploy --no-wallet watchdog-e2e-fake-bitcoin-canister
BITCOIN_CANISTER_ID=$(dfx canister id watchdog-e2e-fake-bitcoin-canister)
if [[ -z "${BITCOIN_CANISTER_ID}" ]]; then
  echo "Failed to create bitcoin canister"
  exit 1
fi

# Deploy watchdog canister.
dfx deploy --no-wallet watchdog --argument "(record {
    bitcoin_network = variant { mainnet };
    blocks_behind_threshold = 2;
    blocks_ahead_threshold = 2;
    min_explorers = 2;
    bitcoin_canister_principal = principal \"${BITCOIN_CANISTER_ID}\";
    delay_before_first_fetch_sec = 1;
    interval_between_fetches_sec = 60;
    fake_explorers_server = opt \"https://${FAKE_EXPLORERS_ADDRESS}\";
})"

# Wait until watchdog fetches the data.
sleep 3

# Check watchdog API access target is enabled.
API_ACCESS_TARGET=$(dfx canister call watchdog get_api_access_target)
# TODO: add code here.
echo "API_ACCESS_TARGET: ${API_ACCESS_TARGET}"

# Check bitcoin_canister API access.
BITCOIN_CANISTER_CONFIG=$(dfx canister call watchdog-e2e-fake-bitcoin-canister get_config)
if [[ "${BITCOIN_CANISTER_CONFIG}" != *"api_access = variant { enabled };"* ]]; then
  echo "Fake bitcoin_canister api_access is not enabled"
  exit 1
fi

# If we made it here without any errors, then we can cleanup safely.
trap - EXIT SIGINT
cleanup
echo "SUCCESS"
