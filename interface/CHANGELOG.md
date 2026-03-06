# ChangeLog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [0.4.0] - 2026-03-06

### Added

- Add `BlockchainInfo` struct for `get_blockchain_info` endpoint return type ([#483](https://github.com/dfinity/bitcoin-canister/pull/483))

- Add `AddressForWrongNetwork` struct used in network validation for addresses in `get_balance` and `get_utxos` endpoints ([#458](https://github.com/dfinity/bitcoin-canister/pull/458))

- Add CanisterArg ([#495](https://github.com/dfinity/bitcoin-canister/pull/495))

## [0.3.0] - 2025-12-09

### Added

- Add a `burn_cycles` field to type `UtxosFilter`.

### Changed

- Remove custom `PartialOrd` implementation for type `Utxo`. This is a breaking change in terms of the semantics.

## [0.2.3] - 2025-10-10

### Added

- Add two APIs: `Fees::testnet` and `Fees::mainnet`.

[0.4.0]: https://github.com/dfinity/bitcoin-canister/compare/ic-btc-interface-0.3.0...ic-btc-interface-0.4.0

[0.3.0]: https://github.com/dfinity/bitcoin-canister/compare/ic-btc-interface-0.2.3...ic-btc-interface-0.3.0

[0.2.3]: https://github.com/dfinity/bitcoin-canister/compare/ic-btc-interface-0.2.2...ic-btc-interface-0.2.3
