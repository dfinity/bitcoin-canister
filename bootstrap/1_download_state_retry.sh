#!/usr/bin/env bash
#
# Script for downloading the Bitcoin state.
set -euo pipefail

source "./utils.sh"

BITCOIN_D="$1/bin/bitcoind"
NETWORK="$2"

validate_network "$NETWORK"

# Create a temporary bitcoin.conf file with the required settings.
CONF_FILE=$(mktemp)
generate_config "$NETWORK" "$CONF_FILE"

$BITCOIN_D -conf="$CONF_FILE" -datadir="$DATA_DIR"
