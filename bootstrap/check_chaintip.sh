#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/utils.sh"

BITCOIN_D=$1/bin/bitcoind
BITCOIN_CLI=$1/bin/bitcoin-cli
NETWORK=$2

validate_network "$NETWORK"

# Kill all background processes on exit.
trap "kill 0" EXIT

CONF_FILE=$(mktemp)
generate_config "$NETWORK" "$CONF_FILE" "networkactive=0"

# Run bitcoind in the background with no network access.
$BITCOIN_D -conf="$CONF_FILE" -datadir="$(pwd)/data" > /dev/null &

# Wait for bitcoind to load.
sleep 30

$BITCOIN_CLI -conf="$CONF_FILE" -datadir="$(pwd)/data" getchaintips
