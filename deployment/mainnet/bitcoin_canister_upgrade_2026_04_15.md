# Proposal to upgrade the Bitcoin canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `e99b58864b3da6c91c04423f91d49352150bd18b`

New compressed Wasm hash: `38943988a0bf168efb236ce4b371eb558004b972cf9eacdd0bd470104131f5a8`

Upgrade args hash: `1c08f4660179db8e87a4a5060ca93ea2f476fed7c25a2bd92f9cf8375eff87d9`

Target canister: `ghsi2-tqaaa-aaaan-aaaca-cai`

Previous Bitcoin proposal: https://dashboard.internetcomputer.org/proposal/140786

---

## Motivation

This upgrade focuses on improving the performance and memory efficiency of the Bitcoin canister.

**Cache unstable blocks in stable memory.** Previously, all unstable blocks (the ~100 most recent blocks that have not yet been finalized) were stored entirely in heap memory. This upgrade moves the block data into a stable-memory cache, keeping only a lightweight `CachedBlock` handle (block hash, difficulty, timetstamp, utxos delta, and fee percentiles) on the heap. This significantly reduces heap memory pressure.

**Cache transaction fee rates.** The `get_current_fee_percentiles` endpoint previously derived transaction fee rates by iterating through all transactions in unstable blocks and looking up each input's value via the `OutPointsCache`. This upgrade pre-computes and caches fee rates per block at block insertion time. At query time, fees are simply collected from the cache, avoiding the expensive repeated computation. Cached fee rates are stored alongside each `CachedBlock` and are automatically cleaned up when blocks become stable. A fallback recomputation path is in place for the first upgrade cycle before the cache is populated.

**Cache UTXO deltas for `get_blockchain_info`.** The `get_blockchain_info` endpoint previously read all transactions in unstable blocks to compute the total UTXO count, which was inefficient. This upgrade caches the net UTXO count change per block (outputs created minus inputs spent) so that `get_blockchain_info` can retrieve the count without re-scanning transactions. Note that during the first upgrade cycle, the UTXO delta cache will be empty, causing a temporary inaccuracy in the reported UTXO count from `get_blockchain_info`. This self-heals as the ~100 unstable blocks are ingested and is deemed acceptable for this non-critical informational endpoint.

**Add benchmarks.** New benchmarks have been added for the `bitcoin_get_balance`, `bitcoin_get_utxos`, `bitcoin_get_current_fee_percentiles`, and `bitcoin_get_block_headers` endpoints to track performance over time.

**Dependency update.** The `ic-cdk` dependency has been bumped to v0.20.0.


## Release Notes

```
git log --format='%C(auto) %h %s' 45b45d816e3f2ae6aefbffc1e5ed6e8e9b51a854..e99b58864b3da6c91c04423f91d49352150bd18b -- canister
e99b588 chore(ic-btc-canister): release/2026-04-15 (#518)
34d43c1 refactor: move block metrics to `CachedBlock` (#516)
b341bb6 feat: cherry pick `dfinity/dogecoin-canister@c947b5c` cache unstable blocks in stable memory (#506)
e58a6ca feat: cache tx fee rates in heap memory (#514)
949c2aa feat: add `utxo_deltas` cache for improved `get_blockchain_info` performance (#513)
8d1d4e0 perf: add benchmarks for `bitcoin_get_*` endpoints (#508)
09d7313 chore: bump ic-cdk to v0.20.0 (#510)
5e68e4a feat(watchdog): use `get_blockchain_info` canister endpoint to retrieve monitored canister height (#484)
913b719 chore(ic-btc-canister): release/2026-03-06 (#502)
 ```

## Upgrade args

```
git fetch
git checkout e99b58864b3da6c91c04423f91d49352150bd18b
didc encode -d canister/candid.did -t '(canister_arg)' '(variant { upgrade })' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout e99b58864b3da6c91c04423f91d49352150bd18b
"./scripts/docker-build" "ic-btc-canister"
sha256sum ./ic-btc-canister.wasm.gz
```