# Proposal to upgrade the Bitcoin canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `60a9338940034536957d50a85444419501b1ecc7`

New compressed Wasm hash: `99ed101afe538f6c08134bb2cfda7ab7b3000a5cc44c65c3bcbf1e5772803f90`

Upgrade args hash: `1c08f4660179db8e87a4a5060ca93ea2f476fed7c25a2bd92f9cf8375eff87d9`

Target canister: `ghsi2-tqaaa-aaaan-aaaca-cai`

Previous Bitcoin proposal: https://dashboard.internetcomputer.org/proposal/141435

---

## Motivation

This upgrade changes the main chain selection algorithm so that the reported main chain no longer shortens when two competing blocks appear at the same height with equal accumulated difficulty and equal depth (a contested tip). Previously, the algorithm truncated to the fork point in this case, decreasing the reported main chain height by one and causing `bitcoin_get_utxos` and `bitcoin_get_balance` to temporarily exclude transactions from both competing blocks. On mainnet, all blocks within a difficulty adjustment period share the same target difficulty, so any competing tip triggered this behavior, which occurs roughly every few weeks when two miners find a block at approximately the same time. With this change, ties on `(accumulated_difficulty, depth)` are broken by keeping the first-received child, matching Bitcoin Core's behavior of staying on whichever chain the node was already following. The tiebreaker only applies while branches are exactly equal — as soon as one branch pulls ahead on either difficulty or depth, it wins outright.


## Release Notes

```
git log --format='%C(auto) %h %s' e99b58864b3da6c91c04423f91d49352150bd18b..60a9338940034536957d50a85444419501b1ecc7 -- canister
60a9338 chore(ic-btc-canister): release/2026-05-27 (#531)
707ef54 chore: clarify main-chain tie-break terminology in comments and naming (#530)
e354549 fix: prevent main chain from decreasing when tip is contested (#521)
 ```

## Upgrade args

```
git fetch
git checkout 60a9338940034536957d50a85444419501b1ecc7
didc encode -d canister/candid.did -t '(canister_arg)' '(variant { upgrade })' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout 60a9338940034536957d50a85444419501b1ecc7
"./scripts/docker-build" "ic-btc-canister"
sha256sum ./ic-btc-canister.wasm.gz
```