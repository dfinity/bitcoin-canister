#!/usr/bin/env bash
#
# Utility functions for Bitcoin scripts.
set -euo pipefail

# Validate the network input.
validate_network() {
    local network=$1
    local valid_networks=("mainnet" "testnet" "testnet4")
    if ! [[ " ${valid_networks[*]} " =~ " $network " ]]; then
        echo "Error: NETWORK must be one of ${valid_networks[*]}."
        exit 1
    fi
}
