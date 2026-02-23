#!/usr/bin/env bash

# Configure dfx.json to use pre-built WASM from wasms/ when present (e.g. in CI).
# When wasms/ is not present (local dev), dfx.json is left unchanged and the build step runs.
use_prebuilt_watchdog_wasm() {
  if [[ -f ../../wasms/watchdog.wasm.gz ]]; then
    sed -i.bak 's|"wasm": "../../target/wasm32-unknown-unknown/release/watchdog.wasm.gz"|"wasm": "../../wasms/watchdog.wasm.gz"|' dfx.json
  fi
}

# Function to deploy the watchdog canister for mainnet bitcoin_canister using pre-built WASM.
deploy_watchdog_canister_bitcoin_mainnet() {
  use_prebuilt_watchdog_wasm
  dfx deploy --no-wallet watchdog --argument "(variant { init = record { target = (variant { bitcoin_mainnet } ) } } )"
}

# Function to get watchdog canister metrics.
get_watchdog_canister_metrics() {
  canister_id=$(dfx canister id watchdog)
  curl "http://$canister_id.raw.localhost:8000/metrics"
}

# Function to check for presence of specific fields in the config.
check_config_fields() {
  CONFIG_FIELDS=(
    "network"
    "blocks_behind_threshold"
    "blocks_ahead_threshold"
    "min_explorers"
    "canister_principal"
    "delay_before_first_fetch_sec"
    "interval_between_fetches_sec"
    "explorers"
  )
  
  config=$(dfx canister call watchdog get_config --query)
  for field in "${CONFIG_FIELDS[@]}"; do
    if ! [[ $config == *"$field = "* ]]; then
      echo "FAIL: $field not found in config of ${0##*/}"
      exit 2
    fi
  done
}

# Function to check for presence of specific fields in the health status.
check_health_status_fields() {
  FIELDS=(
    "height_source"
    "height_target"
    "height_diff"
    "height_status"
    "explorers"
  )
  
  health_status=$(dfx canister call watchdog health_status --query)
  for field in "${FIELDS[@]}"; do
    if ! [[ $health_status == *"$field = "* ]]; then
      echo "FAIL: $field not found in health status of ${0##*/}"
      exit 3
    fi
  done
}

# Function to check if health status data is available.
check_health_status_data() {
  ITERATIONS=15
  DELAY_SEC=2
  has_enough_data=0
  for ((i=1; i<=ITERATIONS; i++))
  do
    health_status=$(dfx canister call watchdog health_status --query)
    if ! [[ $health_status == *"height_status = variant { not_enough_data }"* ]]; then
      has_enough_data=1
      break
    fi
    sleep $DELAY_SEC
  done
  if [ $has_enough_data -eq 0 ]; then
    echo "FAIL: Not enough data in health status of ${0##*/}"
    exit 4
  fi
}

# Function to check for presence of specific names in the metrics.
check_metric_names() {
  METRIC_NAMES=(
    "network"
    "blocks_behind_threshold"
    "blocks_ahead_threshold"
    "min_explorers"
    "canister_height"
    "height_target"
    "height_diff"
    "height_status"
    "api_access_target"
    "explorer_height"
    "available_explorers"
  )

  metrics=$(get_watchdog_canister_metrics)
  for name in "${METRIC_NAMES[@]}"; do
    if ! [[ $metrics == *"$name"* ]]; then
      echo "FAIL: $name not found in metrics of ${0##*/}"
      exit 5
    fi
  done
}
