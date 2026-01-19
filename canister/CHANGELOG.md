# Bitcoin canister changelog

## [release/2025-12-03](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2025-12-03)

### Changed

* chore: upgrade ic-cdk and other dependency versions by @ninegua
  in https://github.com/dfinity/bitcoin-canister/pull/429
* refactor: replace RefCell<Option<T>> with OnceCell<T> by @ninegua
  in https://github.com/dfinity/bitcoin-canister/pull/431
* refactor: make BlockTree generic by @ninegua in https://github.com/dfinity/bitcoin-canister/pull/432
* refactor: use fixed size array for BlockHash type by @ninegua in https://github.com/dfinity/bitcoin-canister/pull/433
* refactor: generic support for BlockTree serialization by @ninegua
  in https://github.com/dfinity/bitcoin-canister/pull/434
* refactor: avoid panic in block tree deserialization by @ninegua
  in https://github.com/dfinity/bitcoin-canister/pull/438
* chore: bump `ic-cdk` to v0.19.0 by @lpahlavi in https://github.com/dfinity/bitcoin-canister/pull/446

### Fixed

* fix: add `burn_cycles` field to `SetConfigRequest` by @lpahlavi
  in https://github.com/dfinity/bitcoin-canister/pull/444
* fix: Remove custom PartialOrd implementation for Utxo type by @ninegua
  in https://github.com/dfinity/bitcoin-canister/pull/451

## [release/2025-10-02](https://github.com/dfinity/bitcoin-canister/releases/edit/release%2F2025-10-02)

### Changed

* refactor(types): remove useless BitcoinAddress::from_str safety check by @mducroux
  in https://github.com/dfinity/bitcoin-canister/pull/408
* chore: remove profiling by @mducroux in https://github.com/dfinity/bitcoin-canister/pull/414
* refactor: validation of bitcoin headers and blocks by @gregorydemay
  in https://github.com/dfinity/bitcoin-canister/pull/419

### Fixed

* fix: Prevent insertion of duplicate unstable blocks by @gregorydemay
  in https://github.com/dfinity/bitcoin-canister/pull/422

## [release/2025-07-02](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2025-07-02)

### Added

* feat: add get_successors request / response metrics for bitcoin canister by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/360
* feat: add get_successors_request_interval histogram metric by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/361
* chore: update get_successors metrics by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/364
* feat: add default fees for mainnet/testnet networks by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/376

### Changed

* chore: rename usage of BlockHeader to Header for bitcoin crate v.0.32.4 update by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/345
* chore: cleanups by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/381
* chore: adjust testnet unstable max depth difference by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/382

### Fixed

* fix: fix header adjustment interval underflow by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/339
* fix: remove `rand` dependency from Bitcoin canister by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/348
* fix: fix compute_next_difficulty and update bootstrap scripts for testnet4 by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/353
* fix: reduce Bitcoin canister logs by skipping full GetSuccessorsResponse by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/359
* fix: get_successors request sends unique hashes by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/363
* chore: fix typo by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/373
* fix: fix typo in testnet fees by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/377
* fix: fix unstable tree block stability check for testnet by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/379
* fix: fix memory leak by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/378
* fix: [EXC-1987] Fix encoding of get_block_headers metrics on Bitcoin canister by @dragoljub-duric
  in https://github.com/dfinity/bitcoin-canister/pull/383
* fix: adaptive max depth limit calculation for unstable blocks tree by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/385
* fix: set a non-zero default stability threshold by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/396
* fix(validation): add timestamp validation check testnet4 by @mducroux
  in https://github.com/dfinity/bitcoin-canister/pull/402
* fix: fix stuck canister after upgrade during block fetch by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/405

## [release/2024-08-30](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2024-08-30)

### Changed

* chore: remove legacy_preupgrade feature by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/319
* chore: upgrade Cargo.lock deps and ic-cdk by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/321

## [release/2024-07-28](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2024-07-28)

### Added

* Adds a new endpoint (`get_block_headers`) as specified in https://github.com/dfinity/interface-spec/pull/298.
* Reduces the maximum number of instructions in heartbeats from 4B to 1B. This helps keep the finalization rate of the
  subnet smooth and improves response times.
* Adds a config for eager evaluation of fees, which is useful for local dev environments.
* Adds the candid interface to the metadata section.
* Improves serialization of unstable blocks when upgrading.

## [release/2024-01-22](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2024-01-22)

### Added

* feat: Periodically burn all the cycles in the Bitcoin canister by @dragoljub-duric
  in https://github.com/dfinity/bitcoin-canister/pull/268

## [release/2023-10-13](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-10-13)

### Added

* feat: Add Non-replicated Queries in the Bitcoin API by @dragoljub-duric
  in https://github.com/dfinity/bitcoin-canister/pull/250

### Fixed

* fix: Use vbyte for the computation of a transaction fee by @AlexandraZapuc
  in https://github.com/dfinity/bitcoin-canister/pull/225
* fix: deserialize `BlockTree` iteratively by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/258
* fix: bound length of chain on testnet by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/261
* fix: make api_access metric an enum by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/222

## [release/2023-08-10](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-08-10)

### Changed

* test: add proptests for computing next header target. by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/223
* chore: unify crate versions by moving them to workspace level by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/229
* chore: update some crate revisions by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/230
* chore: bump up candid to 0.9.1 by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/231
* chore: Implement the `From` trait to access the `Txid` bytes by @THLO
  in https://github.com/dfinity/bitcoin-canister/pull/232
* perf: add benchmark for the get_metrics endpoint. by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/238
* perf: calculate the main chain height more efficiently by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/237
* chore: add additional logs to the canister heartbeat by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/239
* fix: add panic hook to bitcoin canister by @maksymar in https://github.com/dfinity/bitcoin-canister/pull/240
* perf: add benchmark for inserting block headers by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/242
* feat: use `criterion` for running benchmarks by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/243
* perf: make block header validation more efficient by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/241
* refactor: move shared types in `ic-btc-types` crate by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/244
* perf: skip next block headers if they are already inserted. by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/245
* fix: add a bound on the length of the unstable chain in testnet/regtest by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/246
* fix: drop next block headers above a certain instructions threshold by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/247
* chore: [EXC-1379] remove interim code from previous upgrade by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/248

## [release/2023-06-12](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-06-12)

### Added

* chore: expose bitcoin_canister api_access metric #205 by @maksymar

### Changed

* chore: derive Serialize for SetConfigRequest #198 by @ielashi
* chore: enable debug formatter for Config #212 by @maksymar

## [release/2023-04-21](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-04-21)

### Added

* feat: add metric to track if the canister is synced by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/167
* feat: add ic-http simple API for HTTP outcalls on the IC with mocks in tests by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/172

### Changed

* feat: do not respond to requests when not fully synced by @dragoljub-duric
  in https://github.com/dfinity/bitcoin-canister/pull/151
* chore: upgrade dfx to 0.13.1 by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/161
* optimize: cache block hash computations to speed up block insertions by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/164
* chore: upgrade stable structures to version 0.5.2 by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/176
* chore(deps): bump h2 from 0.3.16 to 0.3.17 by @dependabot in https://github.com/dfinity/bitcoin-canister/pull/184

### Fixed

* fix: fix mocking concurrent http requests with transform functions in ic-http by @maksymar
  in https://github.com/dfinity/bitcoin-canister/pull/180
* fix: next block headers validation by @dragoljub-duric in https://github.com/dfinity/bitcoin-canister/pull/175

## [release/2023-03-31](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-03-31)

### Added

* Metric to track stable block insertions by @dragoljub-duric in https://github.com/dfinity/bitcoin-canister/pull/150
* Metric to track unstable block insertions by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/153

### Changed

* feat: use the guard pattern when fetching blocks. by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/154
* chore: upgrade rust to 1.68.0 by @ielashi in https://github.com/dfinity/bitcoin-canister/pull/155

### Fixed

* fix: ignore coinbase transactions when computing fee percentiles. by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/152
* fix: fix bug in retrieving the caller in the set_config endpoint. by @ielashi
  in https://github.com/dfinity/bitcoin-canister/pull/157

## [release/2023-02-23](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-02-23)

### Changed

- Validating timestamps is now consistent with Bitcoin core. Timestamps are validated to ensure they aren't too far in
  the future.
- The fee for each endpoint is now charged in all cases (as opposed to only charging if the input is valid).

### Fixed

- Correctly set the syncing flag in the init method.

## [release/2023-01-30](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-01-30)

- The computation for the number of confirmations of a block has been changed.
  Rather than using the depth of a block as its number of confirmations, the stability
  count of a block is now used as its confirmation count. Using the stability count
  reduces the risk of inconsistencies due to forks. You can read more details here.

## [release/2023-01-19](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2023-01-19)

- Increases stability threshold from 40 to 100.
- Enhancement to fork resolution: Rather than choosing the longest chain as the main chain, the difficulty of the blocks
  in each chain is now taken into account to protect against cases where an attacker manages to feed in a long fork that
  consists of blocks with low difficulty.

## [release/2022-12-20](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2022-12-20)

Includes a number of features and security enhancements:

- Block header validation
- Increasing the stability threshold from 30 to 40
- Flag to enable/disable the API
- Security updates to dependencies

## [release/2022-12-02: Initial Release](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2022-12-02)

- The initial release of the Bitcoin canister.
