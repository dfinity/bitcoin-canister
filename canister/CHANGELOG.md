# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [release/2026-02-18] - 2026-02-18

### Added

- Add `get_blockchain_info` endpoint ([#483](https://github.com/dfinity/bitcoin-canister/pull/483))

- Add network validation for addresses in get_balance and get_utxos requests ([#458](https://github.com/dfinity/bitcoin-canister/pull/458))

- Add canister_arg to initialize and upgrade canister ([#457](https://github.com/dfinity/bitcoin-canister/pull/457))


## [release/2025-12-03] - 2025-12-03

### Changed

- Upgrade ic-cdk and other dependency versions ([#429](https://github.com/dfinity/bitcoin-canister/pull/429))
- Replace RefCell<Option<T>> with OnceCell<T> ([#431](https://github.com/dfinity/bitcoin-canister/pull/431))
- Make BlockTree generic ([#432](https://github.com/dfinity/bitcoin-canister/pull/432))
- Use fixed size array for BlockHash type ([#433](https://github.com/dfinity/bitcoin-canister/pull/433))
- Generic support for BlockTree serialization ([#434](https://github.com/dfinity/bitcoin-canister/pull/434))
- Avoid panic in block tree deserialization ([#438](https://github.com/dfinity/bitcoin-canister/pull/438))
- Bump `ic-cdk` to v0.19.0 ([#446](https://github.com/dfinity/bitcoin-canister/pull/446))

### Fixed

- Add `burn_cycles` field to `SetConfigRequest` ([#444](https://github.com/dfinity/bitcoin-canister/pull/444))
- Remove custom PartialOrd implementation for Utxo type ([#451](https://github.com/dfinity/bitcoin-canister/pull/451))

## [release/2025-10-02] - 2025-10-02

### Changed

- Remove useless BitcoinAddress::from_str safety check ([#408](https://github.com/dfinity/bitcoin-canister/pull/408))
- Remove profiling ([#414](https://github.com/dfinity/bitcoin-canister/pull/414))
- Validation of bitcoin headers and blocks ([#419](https://github.com/dfinity/bitcoin-canister/pull/419))

### Fixed

- Prevent insertion of duplicate unstable blocks ([#422](https://github.com/dfinity/bitcoin-canister/pull/422))

## [release/2025-07-02] - 2025-07-02

### Added

- Add get_successors request/response metrics for bitcoin
  canister ([#360](https://github.com/dfinity/bitcoin-canister/pull/360))
- Add get_successors_request_interval histogram metric ([#361](https://github.com/dfinity/bitcoin-canister/pull/361))
- Update get_successors metrics ([#364](https://github.com/dfinity/bitcoin-canister/pull/364))
- Add default fees for mainnet/testnet networks ([#376](https://github.com/dfinity/bitcoin-canister/pull/376))

### Changed

- Rename usage of BlockHeader to Header for bitcoin crate v.0.32.4
  update ([#345](https://github.com/dfinity/bitcoin-canister/pull/345))
- Cleanups ([#381](https://github.com/dfinity/bitcoin-canister/pull/381))
- Adjust testnet unstable max depth difference ([#382](https://github.com/dfinity/bitcoin-canister/pull/382))

### Fixed

- Fix header adjustment interval underflow ([#339](https://github.com/dfinity/bitcoin-canister/pull/339))
- Remove `rand` dependency from Bitcoin canister ([#348](https://github.com/dfinity/bitcoin-canister/pull/348))
- Fix compute_next_difficulty and update bootstrap scripts for
  testnet4 ([#353](https://github.com/dfinity/bitcoin-canister/pull/353))
- Reduce Bitcoin canister logs by skipping full
  GetSuccessorsResponse ([#359](https://github.com/dfinity/bitcoin-canister/pull/359))
- get_successors request sends unique hashes ([#363](https://github.com/dfinity/bitcoin-canister/pull/363))
- Fix typo ([#373](https://github.com/dfinity/bitcoin-canister/pull/373))
- Fix typo in testnet fees ([#377](https://github.com/dfinity/bitcoin-canister/pull/377))
- Fix unstable tree block stability check for testnet ([#379](https://github.com/dfinity/bitcoin-canister/pull/379))
- Fix memory leak ([#378](https://github.com/dfinity/bitcoin-canister/pull/378))
- Fix encoding of get_block_headers metrics on Bitcoin
  canister ([#383](https://github.com/dfinity/bitcoin-canister/pull/383))
- Adaptive max depth limit calculation for unstable blocks
  tree ([#385](https://github.com/dfinity/bitcoin-canister/pull/385))
- Set a non-zero default stability threshold ([#396](https://github.com/dfinity/bitcoin-canister/pull/396))
- Add timestamp validation check for testnet4 ([#402](https://github.com/dfinity/bitcoin-canister/pull/402))
- Fix stuck canister after upgrade during block fetch ([#405](https://github.com/dfinity/bitcoin-canister/pull/405))

## [release/2024-08-30] - 2024-08-30

### Changed

- Remove legacy_preupgrade feature ([#319](https://github.com/dfinity/bitcoin-canister/pull/319))
- Upgrade Cargo.lock deps and ic-cdk ([#321](https://github.com/dfinity/bitcoin-canister/pull/321))

## [release/2024-07-28] - 2024-07-28

### Added

- Add new endpoint (`get_block_headers`) as specified
  in [interface-spec#298](https://github.com/dfinity/interface-spec/pull/298)
- Add a config for eager evaluation of fees, useful for local dev environments
- Add the candid interface to the metadata section

### Changed

- Reduce the maximum number of instructions in heartbeats from 4B to 1B to keep finalization rate smooth and improve
  response times
- Improve serialization of unstable blocks when upgrading

## [release/2024-01-22] - 2024-01-22

### Added

- Periodically burn all the cycles in the Bitcoin
  canister ([#268](https://github.com/dfinity/bitcoin-canister/pull/268))

## [release/2023-10-13] - 2023-10-13

### Added

- Add Non-replicated Queries in the Bitcoin API ([#250](https://github.com/dfinity/bitcoin-canister/pull/250))

### Fixed

- Use vbyte for the computation of a transaction fee ([#225](https://github.com/dfinity/bitcoin-canister/pull/225))
- Deserialize `BlockTree` iteratively ([#258](https://github.com/dfinity/bitcoin-canister/pull/258))
- Bound length of chain on testnet ([#261](https://github.com/dfinity/bitcoin-canister/pull/261))
- Make api_access metric an enum ([#222](https://github.com/dfinity/bitcoin-canister/pull/222))

## [release/2023-08-10] - 2023-08-10

### Added

- Expose bitcoin_canister api_access metric ([#205](https://github.com/dfinity/bitcoin-canister/pull/205))
- Add benchmark for the get_metrics endpoint ([#238](https://github.com/dfinity/bitcoin-canister/pull/238))
- Add benchmark for inserting block headers ([#242](https://github.com/dfinity/bitcoin-canister/pull/242))
- Use `criterion` for running benchmarks ([#243](https://github.com/dfinity/bitcoin-canister/pull/243))
- Add additional logs to the canister heartbeat ([#239](https://github.com/dfinity/bitcoin-canister/pull/239))
- Add panic hook to bitcoin canister ([#240](https://github.com/dfinity/bitcoin-canister/pull/240))

### Changed

- Add proptests for computing next header target ([#223](https://github.com/dfinity/bitcoin-canister/pull/223))
- Unify crate versions by moving them to workspace level ([#229](https://github.com/dfinity/bitcoin-canister/pull/229))
- Update some crate revisions ([#230](https://github.com/dfinity/bitcoin-canister/pull/230))
- Bump candid to 0.9.1 ([#231](https://github.com/dfinity/bitcoin-canister/pull/231))
- Implement the `From` trait to access the `Txid` bytes ([#232](https://github.com/dfinity/bitcoin-canister/pull/232))
- Calculate the main chain height more efficiently ([#237](https://github.com/dfinity/bitcoin-canister/pull/237))
- Make block header validation more efficient ([#241](https://github.com/dfinity/bitcoin-canister/pull/241))
- Move shared types in `ic-btc-types` crate ([#244](https://github.com/dfinity/bitcoin-canister/pull/244))
- Skip next block headers if they are already inserted ([#245](https://github.com/dfinity/bitcoin-canister/pull/245))
- Remove interim code from previous upgrade ([#248](https://github.com/dfinity/bitcoin-canister/pull/248))

### Fixed

- Add a bound on the length of the unstable chain in
  testnet/regtest ([#246](https://github.com/dfinity/bitcoin-canister/pull/246))
- Drop next block headers above a certain instructions
  threshold ([#247](https://github.com/dfinity/bitcoin-canister/pull/247))

## [release/2023-06-12] - 2023-06-12

### Added

- Expose bitcoin_canister api_access metric ([#205](https://github.com/dfinity/bitcoin-canister/pull/205))

### Changed

- Derive Serialize for SetConfigRequest ([#198](https://github.com/dfinity/bitcoin-canister/pull/198))
- Enable debug formatter for Config ([#212](https://github.com/dfinity/bitcoin-canister/pull/212))

## [release/2023-04-21] - 2023-04-21

### Added

- Add metric to track if the canister is synced ([#167](https://github.com/dfinity/bitcoin-canister/pull/167))
- Add ic-http simple API for HTTP outcalls on the IC with mocks in
  tests ([#172](https://github.com/dfinity/bitcoin-canister/pull/172))

### Changed

- Do not respond to requests when not fully synced ([#151](https://github.com/dfinity/bitcoin-canister/pull/151))
- Upgrade dfx to 0.13.1 ([#161](https://github.com/dfinity/bitcoin-canister/pull/161))
- Cache block hash computations to speed up block
  insertions ([#164](https://github.com/dfinity/bitcoin-canister/pull/164))
- Upgrade stable structures to version 0.5.2 ([#176](https://github.com/dfinity/bitcoin-canister/pull/176))

### Security

- Bump h2 from 0.3.16 to 0.3.17 ([#184](https://github.com/dfinity/bitcoin-canister/pull/184))

### Fixed

- Fix mocking concurrent http requests with transform functions in
  ic-http ([#180](https://github.com/dfinity/bitcoin-canister/pull/180))
- Fix next block headers validation ([#175](https://github.com/dfinity/bitcoin-canister/pull/175))

## [release/2023-03-31] - 2023-03-31

### Added

- Metric to track stable block insertions ([#150](https://github.com/dfinity/bitcoin-canister/pull/150))
- Metric to track unstable block insertions ([#153](https://github.com/dfinity/bitcoin-canister/pull/153))

### Changed

- Use the guard pattern when fetching blocks ([#154](https://github.com/dfinity/bitcoin-canister/pull/154))
- Upgrade rust to 1.68.0 ([#155](https://github.com/dfinity/bitcoin-canister/pull/155))

### Fixed

- Ignore coinbase transactions when computing fee
  percentiles ([#152](https://github.com/dfinity/bitcoin-canister/pull/152))
- Fix bug in retrieving the caller in the set_config
  endpoint ([#157](https://github.com/dfinity/bitcoin-canister/pull/157))

## [release/2023-02-23] - 2023-02-23

### Changed

- Validating timestamps is now consistent with Bitcoin core. Timestamps are validated to ensure they aren't too far in
  the future.
- The fee for each endpoint is now charged in all cases (as opposed to only charging if the input is valid).

### Fixed

- Correctly set the syncing flag in the init method.

## [release/2023-01-30] - 2023-01-30

### Changed

- The computation for the number of confirmations of a block has been changed. Rather than using the depth of a block as
  its number of confirmations, the stability count of a block is now used as its confirmation count. Using the stability
  count reduces the risk of inconsistencies due to forks.

## [release/2023-01-19] - 2023-01-19

### Changed

- Increase stability threshold from 40 to 100.
- Enhancement to fork resolution: Rather than choosing the longest chain as the main chain, the difficulty of the blocks
  in each chain is now taken into account to protect against cases where an attacker manages to feed in a long fork that
  consists of blocks with low difficulty.

## [release/2022-12-20] - 2022-12-20

### Added

- Block header validation
- Flag to enable/disable the API

### Changed

- Increase the stability threshold from 30 to 40

### Security

- Security updates to dependencies

## [release/2022-12-02] - 2022-12-02

### Added

- Initial release of the Bitcoin canister.

[release/2025-12-03]: https://github.com/dfinity/bitcoin-canister/compare/release/2025-10-02...release/2025-12-03

[release/2025-10-02]: https://github.com/dfinity/bitcoin-canister/compare/release/2025-07-02...release/2025-10-02

[release/2025-07-02]: https://github.com/dfinity/bitcoin-canister/compare/release/2024-08-30...release/2025-07-02

[release/2024-08-30]: https://github.com/dfinity/bitcoin-canister/compare/release/2024-07-28...release/2024-08-30

[release/2024-07-28]: https://github.com/dfinity/bitcoin-canister/compare/release/2024-01-22...release/2024-07-28

[release/2024-01-22]: https://github.com/dfinity/bitcoin-canister/compare/release/2023-10-13...release/2024-01-22

[release/2023-10-13]: https://github.com/dfinity/bitcoin-canister/compare/release/2023-08-10...release/2023-10-13

[release/2023-08-10]: https://github.com/dfinity/bitcoin-canister/compare/release/2023-06-12...release/2023-08-10

[release/2023-06-12]: https://github.com/dfinity/bitcoin-canister/compare/release/2023-04-21...release/2023-06-12

[release/2023-04-21]: https://github.com/dfinity/bitcoin-canister/compare/release/2023-03-31...release/2023-04-21

[release/2023-03-31]: https://github.com/dfinity/bitcoin-canister/compare/release/2023-02-23...release/2023-03-31

[release/2023-02-23]: https://github.com/dfinity/bitcoin-canister/compare/release/2023-01-30...release/2023-02-23

[release/2023-01-30]: https://github.com/dfinity/bitcoin-canister/compare/release/2023-01-19...release/2023-01-30

[release/2023-01-19]: https://github.com/dfinity/bitcoin-canister/compare/release/2022-12-20...release/2023-01-19

[release/2022-12-20]: https://github.com/dfinity/bitcoin-canister/compare/release/2022-12-02...release/2022-12-20

[release/2022-12-02]: https://github.com/dfinity/bitcoin-canister/releases/tag/release/2022-12-02

[release/2026-02-18]: https://github.com/dfinity/bitcoin-canister/compare/ic-btc-canister/release/2025-12-03...ic-btc-canister/release/2026-02-18
