# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-06

### Added

- Add timestamp validation check testnet4 ([#402](https://github.com/dfinity/bitcoin-canister/pull/402))

- Add proptests for computing next header target. ([#223](https://github.com/dfinity/bitcoin-canister/pull/223))

### Changed

- Upgrade ic-cdk and other dependency versions ([#429](https://github.com/dfinity/bitcoin-canister/pull/429))

- Validation of bitcoin headers and blocks ([#419](https://github.com/dfinity/bitcoin-canister/pull/419))

- Upgrade bitcoin crate to 0.32.4 for testnet4 support ([#349](https://github.com/dfinity/bitcoin-canister/pull/349))

- Rename usage of BlockHeader to Header for bitcoin crate v.0.32.4
  update ([#345](https://github.com/dfinity/bitcoin-canister/pull/345))

- Make block header validation more efficient ([#241](https://github.com/dfinity/bitcoin-canister/pull/241))

- Compute block difficulty ([#131](https://github.com/dfinity/bitcoin-canister/pull/131))

- Block header validation ([#129](https://github.com/dfinity/bitcoin-canister/pull/129))

- Optimize computing next difficulty ([#128](https://github.com/dfinity/bitcoin-canister/pull/128))

- Move bitcoin validation crate into this repository. ([#92](https://github.com/dfinity/bitcoin-canister/pull/92))

### Fixed

- Fix compute_next_difficulty and update bootstrap scripts for
  testnet4 ([#353](https://github.com/dfinity/bitcoin-canister/pull/353))

- Fix header adjustment interval underflow ([#339](https://github.com/dfinity/bitcoin-canister/pull/339))

- Fix finding next difficulty in chain for testnets ([#221](https://github.com/dfinity/bitcoin-canister/pull/221))

- Validate that the block is < 2 hours from the current
  time. ([#149](https://github.com/dfinity/bitcoin-canister/pull/149))

### Removed

- Remove `rand` dependency from Bitcoin canister ([#348](https://github.com/dfinity/bitcoin-canister/pull/348))

- Remove unneeded block validation checks. ([#127](https://github.com/dfinity/bitcoin-canister/pull/127))

## [0.1.0] - 2024-03-28

- Initial release

[0.2.0]: https://github.com/dfinity/bitcoin-canister/compare/ic-btc-validation-0.1.0...ic-btc-validation-0.2.0
