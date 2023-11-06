# Development Guide

## Release Preparation

The canisters in this repository are deployed in production by submitting proposals to the Internet Computer's [Network Nervous System](https://internetcomputer.org/nns).

Due to limitations in GitHub's release handling, we cannot create separate latest releases for the Bitcoin canister and the Watchdog canister. Therefore, we include both artifacts in each latest release. 

Only after all the expected canisters were deployed the `pre-release` can be turned into a proper `latest release`.

## Steps to Cut a Release

1. Identify the commit for the release, eg. `aff3eef`
2. Draft a new pre-release
    - Click on `Draft a new release` at the [releases page](https://github.com/dfinity/bitcoin-canister/releases), make sure the right commit is selected
    - Create a new tag with the name `release/<yyyy-mm-dd>`
    - Set the title to be `release/<yyyy-mm-dd>`
    - Check the `Set as a pre-release` box to indicate that this release has not been deployed to production yet
    - Add release notes. Github can generate the release notes by clicking on `Generated Release Notes`, modify as needed
3. Prepare canister WASM files and compute their checksums
    - **Note**: there is no guarantee on Mac M1 for reproducible build, preferably use Ubuntu
    ```shell
    # Checkout the repo with a given commit.
    $ git clone https://github.com/dfinity/bitcoin-canister &&\
        cd bitcoin-canister &&\
        git checkout aff3eef  # <- make sure the right commit is provided.

    # Use docker to reproducibly build canister WASMs.
    $ docker build -t canisters .

    # Extract WASM files.
    $ docker run --rm --entrypoint cat canisters /ic-btc-canister.wasm.gz > ic-btc-canister.wasm.gz
    $ docker run --rm --entrypoint cat canisters /watchdog-canister.wasm.gz > watchdog-canister.wasm.gz

    # Compute checksums.
    $ sha256sum *.wasm.gz
    09f5647a45ff6d5d05b2b0ed48613fb2365b5fe6573ba0e901509c39fb9564ac  ic-btc-canister.wasm.gz
    cc58b2a32517f9907f0d3c77bc8c099d0a65d8194a8d9bc0ad7df357ee867a07  watchdog-canister.wasm.gz
    ```
4. Attach the Bitcoin Canister's and Watchdog's WASM to the release notes (and nothing else).
    - Add calculated checksums into release notes
5. Finalize the release once all the expected canisters were upgraded
    - (Optionally) provide links to corresponding NNS proposals
    - Uncheck `Set as a pre-release` box to indicate that it's all deployed
