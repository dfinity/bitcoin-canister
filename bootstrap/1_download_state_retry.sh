#!/usr/bin/env bash
#
# Script for downloading the Bitcoin state.
set -euo pipefail
source "$(dirname "$0")/utils.sh"

# Ensure correct usage.
if [[ $# -ne 2 ]]; then
    echo "Usage: $0 <path-to-bitcoin-dir> <network>"
    exit 1
fi

BITCOIN_D="$1/bin/bitcoind"
NETWORK="$2"

validate_network "$NETWORK"

# Create a temporary bitcoin.conf file with the required settings.
CONF_FILE=$(mktemp)
cat << EOF > "$CONF_FILE"
# Reduce storage requirements by only storing the most recent N MiB of blocks.
prune=5000

# Dummy credentials required by bitcoin-cli.
rpcuser=ic-btc-integration
rpcpassword=QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E=
rpcauth=ic-btc-integration:cdf2741387f3a12438f69092f0fdad8e\$62081498c98bee09a0dce2b30671123fa561932992ce377585e8e08bb0c11dfa
EOF

# Add network-specific configuration if necessary.
case "$NETWORK" in
    "testnet") echo "chain=test" >> "$CONF_FILE" ;;
    "testnet4") echo "chain=testnet4" >> "$CONF_FILE" ;;
esac

$BITCOIN_D -conf="$CONF_FILE" -datadir="$(pwd)/data"
