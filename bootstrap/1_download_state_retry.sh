#!/usr/bin/env bash
#
# Script for downloading the bitcoin state.
set -euo pipefail

source "$(dirname "$0")/utils.sh"

BITCOIN_D=$1/bin/bitcoind
NETWORK=$2

validate_network "$NETWORK"

# Create a bitcoin.conf file that downloads blocks up to the given height.
CONF_FILE=$(mktemp)
generate_config "$NETWORK" "$CONF_FILE"

$BITCOIN_D -conf="$CONF_FILE" -datadir="$(pwd)/data"
