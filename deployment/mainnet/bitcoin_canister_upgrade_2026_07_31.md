# Proposal to upgrade the Bitcoin canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `e34f709551a9d0fbbd63002c0db1a730138f0de6`

New compressed Wasm hash: `9aee15c892d1dbaa9c8daf6805ffbbef0ef09c0d7afd8079d8f4ad8cb85747f1`

Upgrade args hash: `1c08f4660179db8e87a4a5060ca93ea2f476fed7c25a2bd92f9cf8375eff87d9`

Target canister: `ghsi2-tqaaa-aaaan-aaaca-cai`

Previous Bitcoin proposal: https://dashboard.internetcomputer.org/proposal/141982

---

## Motivation
Reduce the number of instructions executed in the heartbeat by removing unnecessary reads. In particular:
- Avoid deep-cloning ingesting_block on every heartbeat.
- Make BlockTree::get_hashes O(n) instead of O(n²).
- Cache unstable-block tip depths, refresh at push/pop.


## Release Notes

```
git log --format='%C(auto) %h %s' 60a9338940034536957d50a85444419501b1ecc7..e34f709551a9d0fbbd63002c0db1a730138f0de6 -- canister
e34f709 perf: DEFI-2957: Cache unstable-block tip depths, refresh at push/pop (#543)
a9b9886 perf: DEFI-2957: Make BlockTree::get_hashes O(n) instead of O(n²) (#541)
b15f421 perf: DEFI-2954: Avoid deep-cloning ingesting_block on every heartbeat (#538)
 ```

## Upgrade args

```
git fetch
git checkout e34f709551a9d0fbbd63002c0db1a730138f0de6
didc encode -d canister/candid.did -t '(canister_arg)' '(variant { upgrade })' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout e34f709551a9d0fbbd63002c0db1a730138f0de6
"./scripts/docker-build" "ic-btc-canister"
sha256sum ./ic-btc-canister.wasm.gz
```
