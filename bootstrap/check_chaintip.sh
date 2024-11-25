#!/usr/bin/env bash
set -euo pipefail

# Ensure correct usage.
if [[ $# -ne 2 ]]; then
    echo "Usage: $0 <path-to-bitcoin-dir> <network>"
    exit 1
fi

BITCOIN_D="$1/bin/bitcoind"
BITCOIN_CLI="$1/bin/bitcoin-cli"
NETWORK="$2"

# Kill all background processes on exit.
trap "kill 0" EXIT

# Validate network input.
VALID_NETWORKS=("mainnet" "testnet" "testnet4")
if ! [[ " ${VALID_NETWORKS[*]} " =~ " $NETWORK " ]]; then
    echo "Error: NETWORK must be one of ${VALID_NETWORKS[*]}."
    exit 1
fi

# Create a temporary bitcoin.conf file with the required settings.
CONF_FILE=$(mktemp)
cat << EOF > "$CONF_FILE"
networkactive=0

# Reduce storage requirements by only storing the most recent N MiB of blocks.
prune=5000

# Dummy credentials required by bitcoin-cli.
rpcuser=ic-btc-integration
rpcpassword=QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E=
rpcauth=ic-btc-integration:cdf2741387f3a12438f69092f0fdad8e\$62081498c98bee09a0dce2b30671123fa561932992ce377585e8e08bb0c11dfa
EOF

# Add network-specific configuration if necessary.
[[ "$NETWORK" == "testnet" ]] && echo "chain=test" >> "$CONF_FILE"
[[ "$NETWORK" == "testnet4" ]] && echo "chain=testnet4" >> "$CONF_FILE"

DATA_DIR="$(pwd)/data"

# Start bitcoind in the background with no network access.
echo "Starting bitcoind for $NETWORK..."
"$BITCOIN_D" -conf="$CONF_FILE" -datadir="$DATA_DIR" > /dev/null &
BITCOIND_PID=$!

# Wait for bitcoind to initialize.
echo "Waiting for bitcoind to load..."
sleep 30

# Get chain tips.
echo "Fetching chain tips for $NETWORK..."
"$BITCOIN_CLI" -conf="$CONF_FILE" -datadir="$DATA_DIR" getchaintips

# Clean up.
kill "$BITCOIND_PID"
wait "$BITCOIND_PID" || true
echo "Done."
