#!/usr/bin/env bash
set -euo pipefail

BITCOIND_DIR=$1
BITCOIN_D=$1/bin/bitcoind
BITCOIN_CLI=$1/bin/bitcoin-cli

# Kill all background processes on exit.
trap "kill 0" EXIT

# Run bitcoind in the background with no network access.
$BITCOIN_D -conf=$(pwd)/bitcoin_2.conf -datadir=$(pwd)/data > /dev/null &

# Wait for bitcoind to load.
sleep 10

$BITCOIN_CLI -conf=$(pwd)/bitcoin.conf -datadir=$(pwd)/data getchaintips
