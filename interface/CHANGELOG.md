# ChangeLog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]


## [0.4.0] - 2026-01-21

### Added

- Add network validation for addresses in get_balance and get_utxos requests ([#458](https://github.com/dfinity/bitcoin-canister/pull/458))


### Changed

- Use only two cliff.toml for semver and date-based version changelog

- Have separate cliff.toml files for each changelog


[0.4.0]: https://github.com/dfinity/bitcoin-canister/compare/0.3.0...0.4.0

## [0.3.0] - 2025-12-09

### Added

- Add a `burn_cycles` field to type `UtxosFilter`.

### Changed

- Remove custom `PartialOrd` implementation for type `Utxo`. This is a breaking change in terms of the semantics.

## [0.2.3] - 2025-10-10

### Added

- Add two APIs: `Fees::testnet` and `Fees::mainnet`.

[unreleased]: https://github.com/dfinity/bitcoin-canister/compare/ic-btc-interface-0.3.0...HEAD
[0.3.0]: https://github.com/dfinity/bitcoin-canister/compare/ic-btc-interface-0.2.3...ic-btc-interface-0.3.0
[0.2.3]: https://github.com/dfinity/bitcoin-canister/compare/ic-btc-interface-0.2.2...ic-btc-interface-0.2.3
