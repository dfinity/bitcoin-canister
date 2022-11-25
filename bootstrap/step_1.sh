#!/usr/bin/env bash
set -euo pipefail

BITCOIN_D=$1/bin/bitcoind
HEIGHT=$2

if [ -d "data" ];
then
    echo "data directory already exists."
    exit 1
fi

# Create a directory to store the blocks.
mkdir data

# Create a bitcoin.conf file that downloads blocks up to the given height.
CONF_FILE=$(mktemp)
cat << EOF > "$CONF_FILE"
# Stop running after reaching the given height in the main chain.
stopatheight=$HEIGHT

# Reduce storage requirements by only storing most recent N MiB of block.
prune=5000

# Dummy credentials that are required by bitcoin-cli.
rpcuser=ic-btc-integration
rpcpassword=QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E=
rpcauth=ic-btc-integration:cdf2741387f3a12438f69092f0fdad8e\$62081498c98bee09a0dce2b30671123fa561932992ce377585e8e08bb0c11dfa
EOF

LOG_FILE=$(mktemp)
echo "Downloading the bitcoin blocks up to height $HEIGHT (output streamed to $LOG_FILE)
This can take several hours..."
$BITCOIN_D -conf="$CONF_FILE" -datadir="$(pwd)/data" > "$LOG_FILE"
echo "Done."

echo "Making a backup of the downloaded state in ./data_bk"
cp -r ./data ./data_bk
echo "Done."
