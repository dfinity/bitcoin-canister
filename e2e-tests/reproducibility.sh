#!/bin/bash

# This script verifies the reproducibility of a Docker build by
# performing the following steps:
# - Build the Docker image twice
# - Copy the WebAssembly (wasm) files from each build
# - Calculate the SHA256 sums of the wasm files
# - Compare the SHA256 sums to check for reproducibility
#
# Example:
# ./e2e-tests/reproducibility.sh Dockerfile

# Verify the argument count
if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <dockerfile>"
  exit 1
fi

# Get the absolute path of the Dockerfile
dockerfile=$(realpath "$1")
dockerfile_dir=$(dirname "$dockerfile")

# Build the Docker image for the first time
echo "Building Docker image (1st build)..."
docker build -t canisters "$dockerfile_dir"

# Create a directory to store the wasm files
mkdir -p wasms

# Extract the wasm files from the first build
docker run --rm -v "./wasms:/wasms" canisters cp /watchdog.wasm.gz /wasms/watchdog.wasm.gz
docker run --rm -v "./wasms:/wasms" canisters cp /uploader.wasm.gz /wasms/uploader.wasm.gz
docker run --rm -v "./wasms:/wasms" canisters cp /ic-btc-canister.wasm.gz /wasms/ic-btc-canister.wasm.gz

# Calculate the SHA256 sums for the first build
echo "Calculating SHA256 sums (1st build)..."
if ! sha256sum1=$(sha256sum "wasms/watchdog.wasm.gz" "wasms/uploader.wasm.gz" "wasms/ic-btc-canister.wasm.gz" 2>&1); then
  echo "ERROR: Failed to calculate SHA256 sums for 1st build"
  echo "$sha256sum1"
  exit 1
fi

# Build the Docker image for the second time
echo "Building Docker image (2nd build)..."
docker build -t canisters "$dockerfile_dir"

# Extract the wasm files from the second build
docker run --rm -v "./wasms:/wasms" canisters cp /watchdog.wasm.gz /wasms/watchdog.wasm.gz
docker run --rm -v "./wasms:/wasms" canisters cp /uploader.wasm.gz /wasms/uploader.wasm.gz
docker run --rm -v "./wasms:/wasms" canisters cp /ic-btc-canister.wasm.gz /wasms/ic-btc-canister.wasm.gz

# Calculate the SHA256 sums for the second build
echo "Calculating SHA256 sums (2nd build)..."
if ! sha256sum2=$(sha256sum "wasms/watchdog.wasm.gz" "wasms/uploader.wasm.gz" "wasms/ic-btc-canister.wasm.gz" 2>&1); then
  echo "ERROR: Failed to calculate SHA256 sums for 2nd build"
  echo "$sha256sum2"
  exit 1
fi

# Compare the SHA256 sums
if [ "$sha256sum1" = "$sha256sum2" ]; then
  echo "SUCCESS: Reproducible build, SHA256 sums match."
  echo "Result SHA256 Sums:"
  echo "$sha256sum1"
  exit 0
else
  echo "FAIL: Non-reproducible build, SHA256 sums differ."
  echo "Result SHA256 Sums 1st Build:"
  echo "$sha256sum1"
  echo "Result SHA256 Sums 2nd Build:"
  echo "$sha256sum2"
  exit 1
fi
