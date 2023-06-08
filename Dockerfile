# Dockerfile: Canister Build Environment
#
# This Dockerfile prepares an environment to build and verify the integrity of 
# these specific WebAssembly canisters:
#  - ic-btc-canister
#  - uploader-canister
#  - watchdog-canister
#
# Each canister is built, compressed, and checksum-verified, ensuring 
# reproducibility and consistency of builds within this isolated setup.
#
# Use the following commands:
# docker build -t canisters .
# docker run --rm --entrypoint cat canisters /ic-btc-canister.wasm.gz > ic-btc-canister.wasm.gz
# docker run --rm --entrypoint cat canisters /uploader-canister.wasm.gz > uploader-canister.wasm.gz
# docker run --rm --entrypoint cat canisters /watchdog-canister.wasm.gz > watchdog-canister.wasm.gz

# The docker image. To update, run `docker pull ubuntu` locally, and update the
# sha256:... accordingly.
FROM ubuntu@sha256:626ffe58f6e7566e00254b638eb7e0f3b11d4da9675088f4781a50ae288f3322

# NOTE: if this version is updated, then the version in rust-toolchain.toml
# should be updated as well.
ARG rust_version=1.68.0

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

# Copy the current directory (containing your source code and build scripts) into the Docker image.
COPY . .

RUN \
    echo "Building bitcoin canister..." && \
    scripts/build-canister.sh ic-btc-canister && \
    cp target/wasm32-unknown-unknown/release/ic-btc-canister.wasm.gz ic-btc-canister.wasm.gz && \
    sha256sum ic-btc-canister.wasm.gz && \

    echo "Building uploader canister..." && \
    scripts/build-canister.sh uploader-canister && \
    cp target/wasm32-unknown-unknown/release/uploader-canister.wasm.gz uploader-canister.wasm.gz && \
    sha256sum uploader-canister.wasm.gz && \

    echo "Building watchdog canister..." && \
    scripts/build-canister.sh watchdog-canister && \
    cp target/wasm32-unknown-unknown/release/watchdog-canister.wasm.gz watchdog-canister.wasm.gz && \
    sha256sum watchdog-canister.wasm.gz
