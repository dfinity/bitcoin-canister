# Use this with
#
# docker build -t canisters .
# docker run --rm --entrypoint cat canisters /ic-btc-canister.wasm > ic-btc-canister.wasm
# docker run --rm --entrypoint cat canisters /uploader-canister.wasm > uploader-canister.wasm

# The docker image. To update, run `docker pull ubuntu` locally, and update the
# sha256:... accordingly.
FROM ubuntu@sha256:626ffe58f6e7566e00254b638eb7e0f3b11d4da9675088f4781a50ae288f3322

# NOTE: if this version is updated, then the version in rust-toolchain.toml
# should be updated as well.
ARG rust_version=1.62.1

ENV TZ=UTC

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone && \
    apt -yq update && \
    apt -yqq install --no-install-recommends curl ca-certificates \
    build-essential pkg-config libssl-dev llvm-dev liblmdb-dev clang cmake \
    git

# Install Rust and Cargo in /opt
ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH

RUN curl --fail https://sh.rustup.rs -sSf \
    | sh -s -- -y --default-toolchain ${rust_version}-x86_64-unknown-linux-gnu --no-modify-path && \
    rustup default ${rust_version}-x86_64-unknown-linux-gnu && \
    rustup target add wasm32-unknown-unknown

ENV PATH=/cargo/bin:$PATH

COPY . .

# Build bitcoin canister
RUN scripts/build-canister.sh ic-btc-canister
RUN cp target/wasm32-unknown-unknown/release/canister.wasm ic-btc-canister.wasm
RUN sha256sum ic-btc-canister.wasm

# Build uploader canister
RUN scripts/build-canister.sh uploader
RUN cp target/wasm32-unknown-unknown/release/uploader-canister.wasm uploader-canister.wasm
RUN sha256sum uploader-canister.wasm
