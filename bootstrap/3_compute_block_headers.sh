#!/usr/bin/env bash
#
# Script for dumping the block headers into a file.
set -euo pipefail

source "$(dirname "$0")/utils.sh"

BITCOIN_D=$1/bin/bitcoind
BITCOIN_CLI=$1/bin/bitcoin-cli
HEIGHT=$2
NETWORK=$3
STABLE_HEIGHT=$((HEIGHT-12))

validate_network "$NETWORK"

# Kill all background processes on exit.
trap "kill 0" EXIT

CONF_FILE=$(mktemp)
generate_config "$NETWORK" "$CONF_FILE" "networkactive=0"

# Delete any previously computed block headers file.
rm -f block_headers

echo "Fetching block headers..."
# Run bitcoind in the background with no network access.
$BITCOIN_D -conf="$CONF_FILE" -datadir="$(pwd)/data" > /dev/null &

# Wait for bitcoind to load.
sleep 30

# Retrieve the block hashes and headers via bitcoin-cli.
for ((height = 0; height <= STABLE_HEIGHT; height++))
do
  BLOCK_HASH=$($BITCOIN_CLI -conf="$CONF_FILE" -datadir="$(pwd)/data" getblockhash "$height")
  BLOCK_HEADER=$($BITCOIN_CLI -conf="$CONF_FILE" -datadir="$(pwd)/data" getblockheader "$BLOCK_HASH" false)

  if [ "$((height % 100))" == 0 ]; then
    echo "Processed $height headers"
  fi

  echo "$BLOCK_HASH,$BLOCK_HEADER" >> block_headers
done

sha256sum block_headers

