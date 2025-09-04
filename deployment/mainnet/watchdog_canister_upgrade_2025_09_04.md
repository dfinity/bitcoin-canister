# Proposal to upgrade the Watchdog canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `292b446a0ec64158eb2c68247530870ff201f274`

New compressed Wasm hash: `579d0ece72da46b0040fc7e395d9c4133bd117e2d2a6f7f6056252ec7ce29eb8`

Upgrade args hash: `cb67b7d883fa8d54e0ef985460a0e1f47f213a27cfa40bdf2e2c68b833c352a4`

Target canister: `gatoo-6iaaa-aaaan-aaacq-cai`

Previous Bitcoin canister proposal: https://dashboard.internetcomputer.org/proposal/127666

---

## Motivation

Upgrade dependencies to their latest versions and add new explorers.

## Release Notes

```
git log --format='%C(auto) %h %s' release/2024-01-22.. -- watchdog
 cbf67c8 chore: update dfx to 0.23 and rust to 1.81 (#372)
 1ec71ee fix: fix min_explorers number for testnet watchdog canister (#365)
 ce710cb feat: migrate watchdog canister to Testnet4 (#352)
 64f3183 fix: remove `rand` dependency from Bitcoin canister (#348)
 2cdc183 chore: add watchdog canister metadata (#329)
 c48571b chore: sort dependencies in Cargo.toml files (#330)
 fcb41ac feat: re-enable tracking api.bitaps.com testnet explorer in watchdog canister (#328)
 c225201 feat: improve height target calculation for watchdog canister (#327)
 f029854 chore: add mainnet explorer bitcoinexplorer.org (#326)
 32102be fix: do not include canbench in production (#317)
 c3ee79b chore: update threshold of watchdog testnet canister (#295)
 009784f chore: upgrade rust from 1.70 to 1.76 (#281)
 1692207 chore: revert a workaround for watchdog_health_status test due to fixed IPv4 dfx support (#280)
 ```

## Upgrade args

```
git fetch
git checkout 292b446a0ec64158eb2c68247530870ff201f274
didc encode '(variant { mainnet })' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout 292b446a0ec64158eb2c68247530870ff201f274
docker build -t canisters .
docker run --rm canisters cat /watchdog.wasm.gz > watchdog.wasm.gz
sha256sum watchdog.wasm.gz
```