#!/bin/bash

# To run it from the root folder:
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

# Create a temporary directory to store the wasm files
tmpdir=$(mktemp -d)

# Extract the wasm files from the first build
docker run --rm -v "$tmpdir:/output" canisters cp /watchdog-canister.wasm.gz /output/watchdog-canister.wasm.gz
docker run --rm -v "$tmpdir:/output" canisters cp /uploader-canister.wasm.gz /output/uploader-canister.wasm.gz
docker run --rm -v "$tmpdir:/output" canisters cp /ic-btc-canister.wasm.gz /output/ic-btc-canister.wasm.gz

# Calculate the SHA256 sums for the first build
echo "Calculating SHA256 sums (1st build)..."
sha256sum1=$(sha256sum "$tmpdir/watchdog-canister.wasm.gz" "$tmpdir/uploader-canister.wasm.gz" "$tmpdir/ic-btc-canister.wasm.gz")

# Build the Docker image for the second time
echo "Building Docker image (2nd build)..."
docker build -t canisters "$dockerfile_dir"

# Extract the wasm files from the second build
docker run --rm -v "$tmpdir:/output" canisters cp /watchdog-canister.wasm.gz /output/watchdog-canister.wasm.gz
docker run --rm -v "$tmpdir:/output" canisters cp /uploader-canister.wasm.gz /output/uploader-canister.wasm.gz
docker run --rm -v "$tmpdir:/output" canisters cp /ic-btc-canister.wasm.gz /output/ic-btc-canister.wasm.gz

# Calculate the SHA256 sums for the second build
echo "Calculating SHA256 sums (2nd build)..."
sha256sum2=$(sha256sum "$tmpdir/watchdog-canister.wasm.gz" "$tmpdir/uploader-canister.wasm.gz" "$tmpdir/ic-btc-canister.wasm.gz")

# Compare the SHA256 sums
if [ "$sha256sum1" = "$sha256sum2" ]; then
  echo "SUCCESS: Reproducible build, SHA256 sums match."
  exit 0
else
  echo "FAIL: Non-reproducible build, SHA256 sums differ."
  exit 1
fi
