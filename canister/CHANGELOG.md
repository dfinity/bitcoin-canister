# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [release/2025-12-03] - 2025-12-03

### Changed

- Upgrade ic-cdk and other dependency versions ([#429](https://github.com/dfinity/bitcoin-canister/pull/429)) - @ninegua
- Replace RefCell<Option<T>> with OnceCell<T> ([#431](https://github.com/dfinity/bitcoin-canister/pull/431)) - @ninegua
- Make BlockTree generic ([#432](https://github.com/dfinity/bitcoin-canister/pull/432)) - @ninegua
- Use fixed size array for BlockHash type ([#433](https://github.com/dfinity/bitcoin-canister/pull/433)) - @ninegua
- Generic support for BlockTree serialization ([#434](https://github.com/dfinity/bitcoin-canister/pull/434)) - @ninegua
- Avoid panic in block tree deserialization ([#438](https://github.com/dfinity/bitcoin-canister/pull/438)) - @ninegua
- Bump `ic-cdk` to v0.19.0 ([#446](https://github.com/dfinity/bitcoin-canister/pull/446)) - @lpahlavi

### Fixed

- Add `burn_cycles` field to `SetConfigRequest` ([#444](https://github.com/dfinity/bitcoin-canister/pull/444)) - @lpahlavi
- Remove custom PartialOrd implementation for Utxo type ([#451](https://github.com/dfinity/bitcoin-canister/pull/451)) - @ninegua

## [release/2025-10-02] - 2025-10-02

### Changed

- Remove useless BitcoinAddress::from_str safety check ([#408](https://github.com/dfinity/bitcoin-canister/pull/408)) - @mducroux
- Remove profiling ([#414](https://github.com/dfinity/bitcoin-canister/pull/414)) - @mducroux
- Validation of bitcoin headers and blocks ([#419](https://github.com/dfinity/bitcoin-canister/pull/419)) - @gregorydemay

### Fixed

- Prevent insertion of duplicate unstable blocks ([#422](https://github.com/dfinity/bitcoin-canister/pull/422)) - @gregorydemay

## [release/2025-07-02] - 2025-07-02

### Added

- Add get_successors request/response metrics for bitcoin canister ([#360](https://github.com/dfinity/bitcoin-canister/pull/360)) - @maksymar
- Add get_successors_request_interval histogram metric ([#361](https://github.com/dfinity/bitcoin-canister/pull/361)) - @maksymar
- Update get_successors metrics ([#364](https://github.com/dfinity/bitcoin-canister/pull/364)) - @maksymar
- Add default fees for mainnet/testnet networks ([#376](https://github.com/dfinity/bitcoin-canister/pull/376)) - @maksymar

### Changed

- Rename usage of BlockHeader to Header for bitcoin crate v.0.32.4 update ([#345](https://github.com/dfinity/bitcoin-canister/pull/345)) - @maksymar
- Cleanups ([#381](https://github.com/dfinity/bitcoin-canister/pull/381)) - @maksymar
- Adjust testnet unstable max depth difference ([#382](https://github.com/dfinity/bitcoin-canister/pull/382)) - @maksymar

### Fixed

- Fix header adjustment interval underflow ([#339](https://github.com/dfinity/bitcoin-canister/pull/339)) - @maksymar
- Remove `rand` dependency from Bitcoin canister ([#348](https://github.com/dfinity/bitcoin-canister/pull/348)) - @maksymar
- Fix compute_next_difficulty and update bootstrap scripts for testnet4 ([#353](https://github.com/dfinity/bitcoin-canister/pull/353)) - @maksymar
- Reduce Bitcoin canister logs by skipping full GetSuccessorsResponse ([#359](https://github.com/dfinity/bitcoin-canister/pull/359)) - @maksymar
- get_successors request sends unique hashes ([#363](https://github.com/dfinity/bitcoin-canister/pull/363)) - @maksymar
- Fix typo ([#373](https://github.com/dfinity/bitcoin-canister/pull/373)) - @maksymar
- Fix typo in testnet fees ([#377](https://github.com/dfinity/bitcoin-canister/pull/377)) - @maksymar
- Fix unstable tree block stability check for testnet ([#379](https://github.com/dfinity/bitcoin-canister/pull/379)) - @maksymar
- Fix memory leak ([#378](https://github.com/dfinity/bitcoin-canister/pull/378)) - @maksymar
- Fix encoding of get_block_headers metrics on Bitcoin canister ([#383](https://github.com/dfinity/bitcoin-canister/pull/383)) - @dragoljub-duric
- Adaptive max depth limit calculation for unstable blocks tree ([#385](https://github.com/dfinity/bitcoin-canister/pull/385)) - @maksymar
- Set a non-zero default stability threshold ([#396](https://github.com/dfinity/bitcoin-canister/pull/396)) - @maksymar
- Add timestamp validation check for testnet4 ([#402](https://github.com/dfinity/bitcoin-canister/pull/402)) - @mducroux
- Fix stuck canister after upgrade during block fetch ([#405](https://github.com/dfinity/bitcoin-canister/pull/405)) - @maksymar

## [release/2024-08-30] - 2024-08-30

### Changed

- Remove legacy_preupgrade feature ([#319](https://github.com/dfinity/bitcoin-canister/pull/319)) - @ielashi
- Upgrade Cargo.lock deps and ic-cdk ([#321](https://github.com/dfinity/bitcoin-canister/pull/321)) - @ielashi

## [release/2024-07-28] - 2024-07-28

### Added

- Add new endpoint (`get_block_headers`) as specified in [interface-spec#298](https://github.com/dfinity/interface-spec/pull/298)
- Add a config for eager evaluation of fees, useful for local dev environments
- Add the candid interface to the metadata section

### Changed

- Reduce the maximum number of instructions in heartbeats from 4B to 1B to keep finalization rate smooth and improve response times
- Improve serialization of unstable blocks when upgrading

## [release/2024-01-22] - 2024-01-22

### Added

- Periodically burn all the cycles in the Bitcoin canister ([#268](https://github.com/dfinity/bitcoin-canister/pull/268)) - @dragoljub-duric

## [release/2023-10-13] - 2023-10-13

### Added

- Add Non-replicated Queries in the Bitcoin API ([#250](https://github.com/dfinity/bitcoin-canister/pull/250)) - @dragoljub-duric

### Fixed

- Use vbyte for the computation of a transaction fee ([#225](https://github.com/dfinity/bitcoin-canister/pull/225)) - @AlexandraZapuc
- Deserialize `BlockTree` iteratively ([#258](https://github.com/dfinity/bitcoin-canister/pull/258)) - @ielashi
- Bound length of chain on testnet ([#261](https://github.com/dfinity/bitcoin-canister/pull/261)) - @ielashi
- Make api_access metric an enum ([#222](https://github.com/dfinity/bitcoin-canister/pull/222)) - @maksymar

## [release/2023-08-10] - 2023-08-10

### Added

- Expose bitcoin_canister api_access metric ([#205](https://github.com/dfinity/bitcoin-canister/pull/205)) - @maksymar
- Add benchmark for the get_metrics endpoint ([#238](https://github.com/dfinity/bitcoin-canister/pull/238)) - @ielashi
- Add benchmark for inserting block headers ([#242](https://github.com/dfinity/bitcoin-canister/pull/242)) - @ielashi
- Use `criterion` for running benchmarks ([#243](https://github.com/dfinity/bitcoin-canister/pull/243)) - @ielashi
- Add additional logs to the canister heartbeat ([#239](https://github.com/dfinity/bitcoin-canister/pull/239)) - @ielashi
- Add panic hook to bitcoin canister ([#240](https://github.com/dfinity/bitcoin-canister/pull/240)) - @maksymar

### Changed

- Add proptests for computing next header target ([#223](https://github.com/dfinity/bitcoin-canister/pull/223)) - @ielashi
- Unify crate versions by moving them to workspace level ([#229](https://github.com/dfinity/bitcoin-canister/pull/229)) - @maksymar
- Update some crate revisions ([#230](https://github.com/dfinity/bitcoin-canister/pull/230)) - @maksymar
- Bump candid to 0.9.1 ([#231](https://github.com/dfinity/bitcoin-canister/pull/231)) - @maksymar
- Implement the `From` trait to access the `Txid` bytes ([#232](https://github.com/dfinity/bitcoin-canister/pull/232)) - @THLO
- Calculate the main chain height more efficiently ([#237](https://github.com/dfinity/bitcoin-canister/pull/237)) - @ielashi
- Make block header validation more efficient ([#241](https://github.com/dfinity/bitcoin-canister/pull/241)) - @ielashi
- Move shared types in `ic-btc-types` crate ([#244](https://github.com/dfinity/bitcoin-canister/pull/244)) - @ielashi
- Skip next block headers if they are already inserted ([#245](https://github.com/dfinity/bitcoin-canister/pull/245)) - @ielashi
- Remove interim code from previous upgrade ([#248](https://github.com/dfinity/bitcoin-canister/pull/248)) - @ielashi

### Fixed

- Add a bound on the length of the unstable chain in testnet/regtest ([#246](https://github.com/dfinity/bitcoin-canister/pull/246)) - @ielashi
- Drop next block headers above a certain instructions threshold ([#247](https://github.com/dfinity/bitcoin-canister/pull/247)) - @ielashi

## [release/2023-06-12] - 2023-06-12

### Added

- Expose bitcoin_canister api_access metric ([#205](https://github.com/dfinity/bitcoin-canister/pull/205)) - @maksymar

### Changed

- Derive Serialize for SetConfigRequest ([#198](https://github.com/dfinity/bitcoin-canister/pull/198)) - @ielashi
- Enable debug formatter for Config ([#212](https://github.com/dfinity/bitcoin-canister/pull/212)) - @maksymar

## [release/2023-04-21] - 2023-04-21

### Added

- Add metric to track if the canister is synced ([#167](https://github.com/dfinity/bitcoin-canister/pull/167)) - @ielashi
- Add ic-http simple API for HTTP outcalls on the IC with mocks in tests ([#172](https://github.com/dfinity/bitcoin-canister/pull/172)) - @maksymar

### Changed

- Do not respond to requests when not fully synced ([#151](https://github.com/dfinity/bitcoin-canister/pull/151)) - @dragoljub-duric
- Upgrade dfx to 0.13.1 ([#161](https://github.com/dfinity/bitcoin-canister/pull/161)) - @ielashi
- Cache block hash computations to speed up block insertions ([#164](https://github.com/dfinity/bitcoin-canister/pull/164)) - @ielashi
- Upgrade stable structures to version 0.5.2 ([#176](https://github.com/dfinity/bitcoin-canister/pull/176)) - @ielashi

### Security

- Bump h2 from 0.3.16 to 0.3.17 ([#184](https://github.com/dfinity/bitcoin-canister/pull/184)) - @dependabot

### Fixed

- Fix mocking concurrent http requests with transform functions in ic-http ([#180](https://github.com/dfinity/bitcoin-canister/pull/180)) - @maksymar
- Fix next block headers validation ([#175](https://github.com/dfinity/bitcoin-canister/pull/175)) - @dragoljub-duric

## [release/2023-03-31] - 2023-03-31

### Added

- Metric to track stable block insertions ([#150](https://github.com/dfinity/bitcoin-canister/pull/150)) - @dragoljub-duric
- Metric to track unstable block insertions ([#153](https://github.com/dfinity/bitcoin-canister/pull/153)) - @ielashi

### Changed

- Use the guard pattern when fetching blocks ([#154](https://github.com/dfinity/bitcoin-canister/pull/154)) - @ielashi
- Upgrade rust to 1.68.0 ([#155](https://github.com/dfinity/bitcoin-canister/pull/155)) - @ielashi

### Fixed

- Ignore coinbase transactions when computing fee percentiles ([#152](https://github.com/dfinity/bitcoin-canister/pull/152)) - @ielashi
- Fix bug in retrieving the caller in the set_config endpoint ([#157](https://github.com/dfinity/bitcoin-canister/pull/157)) - @ielashi

## [release/2023-02-23] - 2023-02-23

### Changed

- Validating timestamps is now consistent with Bitcoin core. Timestamps are validated to ensure they aren't too far in the future.
- The fee for each endpoint is now charged in all cases (as opposed to only charging if the input is valid).

### Fixed

- Correctly set the syncing flag in the init method.

## [release/2023-01-30] - 2023-01-30

### Changed

- The computation for the number of confirmations of a block has been changed. Rather than using the depth of a block as its number of confirmations, the stability count of a block is now used as its confirmation count. Using the stability count reduces the risk of inconsistencies due to forks.

## [release/2023-01-19] - 2023-01-19

### Changed

- Increase stability threshold from 40 to 100.
- Enhancement to fork resolution: Rather than choosing the longest chain as the main chain, the difficulty of the blocks in each chain is now taken into account to protect against cases where an attacker manages to feed in a long fork that consists of blocks with low difficulty.

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

[Unreleased]: https://github.com/dfinity/bitcoin-canister/compare/release/2025-12-03...HEAD
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
