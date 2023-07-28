# Development Guide

## Preparing Releases

The canisters in this repository are deployed in production by submitting proposals to the Internet Computer's [Network Nervous System](https://internetcomputer.org/nns).

Below are the steps needed to cut a release:

### Bitcoin Canister

1. Locally, from the same commit as the release is intended, build the canisters by running `docker build -t canisters .`.
2. Extract the wasm of the Bitcoin Canister by running `docker run --rm --entrypoint cat canisters /ic-btc-canister.wasm.gz > ic-btc-canister.wasm.gz`.
3. Compute the checksum of the canister by running `sha256sum ic-btc-canister.wasm.gz`.
4. On the [releases page](https://github.com/dfinity/bitcoin-canister/releases), click on "Draft a new release". Make sure the right commit is selected.
5. Create a new tag with the name: `release/<yyyy-mm-dd>`.
6. Set the title to be: `Bitcoin Canister Release (<yyyy-mm-dd>)`.
7. Add release notes. Github can generate the release notes by clicking on "Generated Release Notes". Modify as needed.
8. Add the commit hash of the Bitcoin Canister's wasm from step 3 to the release notes.
9. Attach the Bitcoin Canister's wasm to the release notes (and nothing else).
10. Check the "Set as a pre-release" box to indicate that this release hasn't been deployed to production yet.
11. Once the Bitcoin mainnet canister is upgraded to this version, uncheck the "Set as a pre-release" box to indicate that it's deployed.

### Watchdog Canister

1. Locally, from the same commit as the release is intended, build the canisters by running `docker build -t canisters .`.
2. Extract the wasm of the Watchdog Canister by running `docker run --rm --entrypoint cat canisters /watchdog-canister.wasm.gz > watchdog-canister.wasm.gz`.
3. Compute the checksum of the canister by running `sha256sum watchdog-canister.wasm.gz`.
4. On the [releases page](https://github.com/dfinity/bitcoin-canister/releases), click on "Draft a new release". Make sure the right commit is selected.
5. Create a new tag with the name: `watchdog/release/<yyyy-mm-dd>`.
6. Set the title to be: `Watchdog Canister Release (<yyyy-mm-dd>)`.
7. Add release notes. Github can generate the release notes by clicking on "Generated Release Notes". Modify as needed.
8. Add the commit hash of the Watchdog Canister's wasm from step 3 to the release notes.
9. Attach the Watchdog Canister's wasm to the release notes (and nothing else).
10. Check the "Set as a pre-release" box to indicate that this release hasn't been deployed to production yet.
11. Once the Watchdog for the Bitcoin mainnet canister is upgraded to this version, uncheck the "Set as a pre-release" box to indicate that it's deployed.
