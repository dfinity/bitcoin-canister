# ChangeLog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [0.4.0] - 2026-03-06

### Added

- Add most accumulated difficulty criterion in main chain selection ([#490](https://github.com/dfinity/bitcoin-canister/pull/490))

- Add `get_blockchain_info` endpoint ([#483](https://github.com/dfinity/bitcoin-canister/pull/483))

- Add network validation for addresses in get_balance and get_utxos requests ([#458](https://github.com/dfinity/bitcoin-canister/pull/458))


### Changed

- Move CanisterArg to ic-btc-interface ([#495](https://github.com/dfinity/bitcoin-canister/pull/495))

- Release plz ([#464](https://github.com/dfinity/bitcoin-canister/pull/464))


[0.4.0]: https://github.com/dfinity/bitcoin-canister/compare/0.3.0...0.4.0

## [0.3.0] - 2025-12-09

### Added

- Add a `burn_cycles` field to type `UtxosFilter`.

### Changed

- Remove custom `PartialOrd` implementation for type `Utxo`. This is a breaking change in terms of the semantics.

## [0.2.3] - 2025-10-10

### Added

- Add two APIs: `Fees::testnet` and `Fees::mainnet`.

[0.3.0]: https://github.com/dfinity/bitcoin-canister/compare/ic-btc-interface-0.2.3...ic-btc-interface-0.3.0

[0.2.3]: https://github.com/dfinity/bitcoin-canister/compare/ic-btc-interface-0.2.2...ic-btc-interface-0.2.3
