#!/usr/bin/env bash
set -euo pipefail

BITCOIN_D=$1/bin/bitcoind
BITCOIN_CLI=$1/bin/bitcoin-cli

# Kill all background processes on exit.
trap "kill 0" EXIT

CONF_FILE=$(mktemp)
cat <<- "EOF" > "$CONF_FILE"
networkactive=0

# Reduce storage requirements by only storing most recent N MiB of block.
prune=5000

# Dummy credentials that are required by `bitcoin-cli`.
rpcuser=ic-btc-integration
rpcpassword=QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E=
rpcauth=ic-btc-integration:cdf2741387f3a12438f69092f0fdad8e$62081498c98bee09a0dce2b30671123fa561932992ce377585e8e08bb0c11dfa
EOF

# Run bitcoind in the background with no network access.
$BITCOIN_D -conf="$CONF_FILE" -datadir="$(pwd)/data" > /dev/null &

# Wait for bitcoind to load.
sleep 30

$BITCOIN_CLI -conf="$CONF_FILE" -datadir="$(pwd)/data" getchaintips
