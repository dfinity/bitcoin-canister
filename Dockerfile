# Dockerfile: Canister Build Environment
#
# This Dockerfile prepares an environment to build and verify the integrity of 
# these specific WebAssembly canisters:
#  - ic-btc-canister
#  - uploader
#  - watchdog
#
# Each canister is built, compressed, and checksum-verified, ensuring 
# reproducibility and consistency of builds within this isolated setup.
#
# Use the following commands:
#
# docker build -t canisters .
# or
# docker build --build-arg CHUNK_HASHES_PATH=/bootstrap/chunk_hashes.txt  -t canisters .
#
# docker run --rm --entrypoint cat canisters /ic-btc-canister.wasm.gz > ic-btc-canister.wasm.gz
# docker run --rm --entrypoint cat canisters /uploader.wasm.gz > uploader.wasm.gz
# docker run --rm --entrypoint cat canisters /watchdog.wasm.gz > watchdog.wasm.gz
#
# sha256sum ic-btc-canister.wasm.gz
# sha256sum uploader.wasm.gz
# sha256sum watchdog.wasm.gz

# The docker image. To update, run `docker pull ubuntu` locally, and update the
# sha256:... accordingly.
FROM ubuntu@sha256:626ffe58f6e7566e00254b638eb7e0f3b11d4da9675088f4781a50ae288f3322

# NOTE: if this version is updated, then the version in rust-toolchain.toml
# should be updated as well.
ARG rust_version=1.90.0
ARG CHUNK_HASHES_PATH
ARG BTC_FEATURES=""

# Setting the timezone and installing the necessary dependencies
ENV TZ=UTC

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone && \
    apt -yq update && \
    apt -yqq install --no-install-recommends curl ca-certificates \
    build-essential pkg-config libssl-dev llvm-dev liblmdb-dev clang cmake \
    git && \
    # Package cleanup to reduce image size.
    rm -rf /var/lib/apt/lists/*

# Install Rust and Cargo in /opt
ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH

RUN curl --fail https://sh.rustup.rs -sSf \
    | sh -s -- -y --default-toolchain ${rust_version}-x86_64-unknown-linux-gnu --no-modify-path && \
    rustup default ${rust_version}-x86_64-unknown-linux-gnu && \
    rustup target add wasm32-unknown-unknown

ENV PATH=/cargo/bin:$PATH

# Copy the current directory (containing source code and build scripts) into the Docker image.
COPY . .

# Building bitcoin canister...
RUN scripts/build-canister.sh ic-btc-canister ${BTC_FEATURES} && \
    cp target/wasm32-unknown-unknown/release/ic-btc-canister.wasm.gz ic-btc-canister.wasm.gz

# Set the path to chunk hashes if specified (for including it in the uploader canister)
RUN if [ -n "$CHUNK_HASHES_PATH" ]; then export CHUNK_HASHES_PATH="$CHUNK_HASHES_PATH"; fi

# Building uploader canister...
RUN scripts/build-canister.sh uploader "" "release-lto" && \
    cp target/wasm32-unknown-unknown/release-lto/uploader.wasm.gz uploader.wasm.gz

# Building watchdog canister...
RUN scripts/build-canister.sh watchdog && \
    cp target/wasm32-unknown-unknown/release/watchdog.wasm.gz watchdog.wasm.gz
