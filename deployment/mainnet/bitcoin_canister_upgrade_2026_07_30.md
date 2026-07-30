# Proposal to upgrade the Bitcoin canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `4a7f02163277b24341f2f84cd02b1c20a22e7aeb`

New compressed Wasm hash: `9aee15c892d1dbaa9c8daf6805ffbbef0ef09c0d7afd8079d8f4ad8cb85747f1`

Upgrade args hash: `1c08f4660179db8e87a4a5060ca93ea2f476fed7c25a2bd92f9cf8375eff87d9`

Target canister: `ghsi2-tqaaa-aaaan-aaaca-cai`

Previous Bitcoin proposal: https://dashboard.internetcomputer.org/proposal/141982

---

## Motivation
This upgrade reduces the Bitcoin canister's per-heartbeat cycle burn, which grew
substantially once replicas enabled the deterministic memory tracker (DMT). DMT
charges instructions for every page access, including reads, so work that was
effectively free before - repeatedly re-reading and re-deriving data that does
not change between heartbeat rounds - now competes with block ingestion for the
same instruction budget. On mainnet staging this was enough to stall sync
progress.

Three sources of such redundant per-round work are removed:

- The block-ingestion path no longer deep-clones and compares the entire
  in-progress block on every heartbeat just to detect whether ingestion made
  progress; the same answer is derived directly from the slicing result.
- Collecting the hashes of unstable blocks for the `get_successors` request is
  now a single traversal into one accumulator instead of concatenating a fresh
  vector per node, which was quadratic in the size of the unstable block tree.
- Unstable-block tip depths, previously recomputed on every heartbeat for a
  diagnostic metric, are cached and refreshed only when a block is pushed or
  popped (roughly every ten minutes), with the cache rebuilt on upgrade for
  state written by an older version.

Together these cut the synchronous per-round heartbeat work from roughly 809K to
roughly 47K instructions (about ~94%) in the `heartbeat_steady_state_synced`
benchmark. There are no interface or behavioural changes: metric cadence and
values are preserved, and the order of hashes the heartbeat relies on is
unchanged.


## Release Notes

```
git log --format='%C(auto) %h %s' 60a9338940034536957d50a85444419501b1ecc7..4a7f02163277b24341f2f84cd02b1c20a22e7aeb -- canister
4a7f021 chore(ic-btc-canister): release/2026-07-30 (#548)
e34f709 perf: DEFI-2957: Cache unstable-block tip depths, refresh at push/pop (#543)
a9b9886 perf: DEFI-2957: Make BlockTree::get_hashes O(n) instead of O(n²) (#541)
b15f421 perf: DEFI-2954: Avoid deep-cloning ingesting_block on every heartbeat (#538)
 ```

## Upgrade args

```
git fetch
git checkout 4a7f02163277b24341f2f84cd02b1c20a22e7aeb
didc encode -d canister/candid.did -t '(canister_arg)' '(variant { upgrade })' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout 4a7f02163277b24341f2f84cd02b1c20a22e7aeb
"./scripts/docker-build" "ic-btc-canister"
sha256sum ./ic-btc-canister.wasm.gz
```
