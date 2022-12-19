#!/usr/bin/env bash
#
# Script for dumping the block headers into a file.
set -euo pipefail

BITCOIN_D=$1/bin/bitcoind
BITCOIN_CLI=$1/bin/bitcoin-cli
HEIGHT=$2
NETWORK=$3
STABLE_HEIGHT=$((HEIGHT-12))

# Kill all background processes on exit.
trap "kill 0" EXIT

if ! [[ "$NETWORK" == "mainnet" || "$NETWORK" == "testnet" ]]; then
    echo "NETWORK must be set to either 'mainnet' or 'testnet'"
    false
fi

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

# Configure bitcoin.conf to connect to the testnet network if needed.
if [[ "$NETWORK" == "testnet" ]]; then
    echo "chain=test" >> "$CONF_FILE"
fi

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

