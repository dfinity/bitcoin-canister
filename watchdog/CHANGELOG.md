# Watchdog canister changelog

## UNRELEASED

Features:
- fix: fix min_explorers number for testnet watchdog canister ([#365](https://github.com/dfinity/bitcoin-canister/pull/365))
- feat: migrate watchdog canister to Testnet4 ([#352](https://github.com/dfinity/bitcoin-canister/pull/352))
- fix: remove `rand` dependency from Bitcoin canister ([#348](https://github.com/dfinity/bitcoin-canister/pull/348))
- chore: add watchdog canister metadata ([#329](https://github.com/dfinity/bitcoin-canister/pull/329)) by @maksymar
- feat: re-enable tracking `api.bitaps.com` `testnet` explorer in watchdog canister ([#328](https://github.com/dfinity/bitcoin-canister/pull/328)) by @maksymar
- feat: improve height target calculation for watchdog canister ([#327](https://github.com/dfinity/bitcoin-canister/pull/327)) by @maksymar
- chore: add mainnet explorer `bitcoinexplorer.org` ([#326](https://github.com/dfinity/bitcoin-canister/pull/326)) by @maksymar
- chore: update threshold of watchdog `testnet` canister ([#295](https://github.com/dfinity/bitcoin-canister/pull/295)) by @islam.elashi

Chores:
- chore: add canister_id.json and docker build script ([#409](https://github.com/dfinity/bitcoin-canister/pull/409))
- chore: update dfx to 0.23 and rust to 1.81 ([#372](https://github.com/dfinity/bitcoin-canister/pull/372))
- chore: sort dependencies in `Cargo.toml` files ([#330](https://github.com/dfinity/bitcoin-canister/pull/330)) by @maksymar
- fix: do not include canbench in production ([#317](https://github.com/dfinity/bitcoin-canister/pull/317)) by @islam.elashi
- chore: upgrade rust from `1.70` to `1.76` ([#281](https://github.com/dfinity/bitcoin-canister/pull/281)) by @islam.elashi
- chore: revert a workaround for watchdog_health_status test due to fixed IPv4 dfx support ([#280](https://github.com/dfinity/bitcoin-canister/pull/280)) by @maksymar

## [2024-01-22 Release](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2024-01-22)

- make `testnet` watchdog canister less sensitive

### Proposals

- 2024-01-27 Testnet [127121](https://dashboard.internetcomputer.org/proposal/127121)
- 2024-02-05 Mainnet [127666](https://dashboard.internetcomputer.org/proposal/127666)


## [2023-10-13 Release](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-10-13)

- improve `api_access` metrics

### Proposals

- 2023-10-23 Testnet [125316](https://dashboard.internetcomputer.org/proposal/125316)
- 2023-10-26 Mainnet [125325](https://dashboard.internetcomputer.org/proposal/125325)


## [2023-06-12 Release](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-06-12)

- integration and enhancement of the watchdog canister

### Proposals

- 2023-06-24 Testnet [123101](https://dashboard.internetcomputer.org/proposal/123101)
- 2023-06-24 Mainnet [123106](https://dashboard.internetcomputer.org/proposal/123106)


## [2023-04-21 Release](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-04-21)

- create watchdog canister
