#!/bin/bash

CANISTER_ID="g4xu7-jiaaa-aaaan-aaaaq-cai"

# Function to fetch logs and filter out new lines
fetch_and_filter_logs() {
    # Fetch logs
    new_logs=$(dfx canister logs --network testnet $CANISTER_ID)

    # Compare with previous logs to find new ones
    while IFS= read -r line; do
        if [[ ! "${previous_logs[*]}" =~ "$line" ]]; then
            echo "$line"
        fi
    done <<< "$new_logs"

    # Update previous logs
    previous_logs=("$new_logs")
}

# Initial fetch and filter
fetch_and_filter_logs

# Infinite loop to continuously fetch and filter logs
while true; do
    fetch_and_filter_logs
    sleep 0.1
done
