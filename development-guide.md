# Development Guide

## Preparing Releases

The canisters in this repository are deployed in production by submitting proposals to the Internet Computer's [Network Nervous System](https://internetcomputer.org/nns).

Below are the steps needed to cut a release:

1. Locally, from the same commit as the release is intended, build the canisters by running `docker build -t canisters .`.
2. Extract the wasm of the Bitcoin Canister by running `docker run --rm --entrypoint cat canisters /ic-btc-canister.wasm.gz > ic-btc-canister.wasm.gz`.
3. Extract the wasm of the Watchdog Canister by running `docker run --rm --entrypoint cat canisters /watchdog-canister.wasm.gz > watchdog-canister.wasm.gz`.
4. Compute the checksum of the canister by running `sha256sum ic-btc-canister.wasm.gz`.
5. Compute the checksum of the canister by running `sha256sum watchdog-canister.wasm.gz`.
6. On the [releases page](https://github.com/dfinity/bitcoin-canister/releases), click on "Draft a new release". Make sure the right commit is selected.
7. Create a new tag with the name: `release/<yyyy-mm-dd>`.
8. Set the title to be: `Release (<yyyy-mm-dd>)`.
9. Add release notes. Github can generate the release notes by clicking on "Generated Release Notes". Modify as needed.
10. Attach the Bitcoin Canister's and Watchdog's wasm to the release notes (and nothing else).
11. Check the "Set as a pre-release" box to indicate that this release hasn't been deployed to production yet.
12. Once the Bitcoin mainnet canister and watchdog mainnet is upgraded to this version, uncheck the "Set as a pre-release" box to indicate that it's deployed.
