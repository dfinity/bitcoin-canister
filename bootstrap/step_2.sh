#!/usr/bin/env bash
set -euo pipefail

BITCOIN_D=$1/bin/bitcoind
BITCOIN_CLI=$1/bin/bitcoin-cli
HEIGHT=$2

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

echo "Preparing the unstable blocks..."
# Run bitcoind in the background with no network access.
$BITCOIN_D -conf="$CONF_FILE" -datadir="$(pwd)/data" > /dev/null &

# Wait for bitcoind to load.
sleep 10

STABLE_HEIGHT=$((HEIGHT-12))

echo "Getting block hash at height $((STABLE_HEIGHT+1))"
BLOCK_HASH_1=$($BITCOIN_CLI -conf="$CONF_FILE" -datadir="$(pwd)/data" getblockhash $((STABLE_HEIGHT+1)))
echo "Hash: $BLOCK_HASH_1"

echo "Getting block hash at height $((STABLE_HEIGHT+2))"
BLOCK_HASH_2=$($BITCOIN_CLI -conf="$CONF_FILE" -datadir="$(pwd)/data" getblockhash $((STABLE_HEIGHT+2)))
echo "Hash: $BLOCK_HASH_2"

$BITCOIN_CLI -conf="$CONF_FILE" getblock "$BLOCK_HASH_1" 0 > unstable_blocks
$BITCOIN_CLI -conf="$CONF_FILE" getblock "$BLOCK_HASH_2" 0 >> unstable_blocks

echo "Invalidating unstable blocks..."
$BITCOIN_CLI -conf="$CONF_FILE" invalidateblock "$BLOCK_HASH_1"

echo "Computing checksum of unstable blocks..."
sha256sum unstable_blocks
echo "Done."
