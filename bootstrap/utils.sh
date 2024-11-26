#!/usr/bin/env bash
#
# Utility functions for Bitcoin scripts.
set -euo pipefail

# Shared directory for data storage.
DATA_DIR="$(pwd)/data"
BACKUP_DIR="$(pwd)/data_bk"
# Files.
UNSTABLE_BLOCKS_FILE="$(pwd)/unstable_blocks"
BLOCK_HEADERS_FILE="$(pwd)/block_headers"
UTXO_DUMP="$(pwd)/utxodump.csv"
UTXO_DUMP_SHUFFLED="$(pwd)/utxodump_shuffled.csv"
# Canister state.
CANISTER_STATE_DIR="$(pwd)/canister_state"
CANISTER_STATE_FILE="$(pwd)/canister_state.bin"

# Validate the network input.
validate_network() {
    local network=$1
    local valid_networks=("mainnet" "testnet")

    for valid_network in "${valid_networks[@]}"; do
        if [[ "$network" == "$valid_network" ]]; then
            # Network is valid
            return 0
        fi
    done

    echo "Error: NETWORK must be one of [ ${valid_networks[*]} ]."
    exit 1
}

# Generate the Bitcoin configuration file with optional parameters.
generate_config() {
    local network=$1
    local conf_file=$2
    shift 2
    local additional_params=("$@")  # Collect additional flags.

    # Basic configuration.
    cat << EOF > "$conf_file"
# Reduce storage requirements by only storing the most recent N MiB of blocks.
prune=5000

# Dummy credentials required by bitcoin-cli.
rpcuser=ic-btc-integration
rpcpassword=QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E=
rpcauth=ic-btc-integration:cdf2741387f3a12438f69092f0fdad8e\$62081498c98bee09a0dce2b30671123fa561932992ce377585e8e08bb0c11dfa
EOF

    # Add network-specific settings.
    case "$network" in
        "mainnet") echo "# Mainnet settings" >> "$conf_file" ;;
        "testnet") echo "chain=test" >> "$conf_file" ;;
    esac

    # Add additional parameters.
    for param in "${additional_params[@]}"; do
        echo "$param" >> "$conf_file"
    done
}
