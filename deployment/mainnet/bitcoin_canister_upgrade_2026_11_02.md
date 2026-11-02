# Proposal to upgrade the Bitcoin canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `e21267ebd071a9f3745e04424d3293b76c96589c`

New compressed Wasm hash: `b539b2cef300cf2f30ab7080c1fcb8319ad5de04a4c7a8790f6050af4e06e4e8`

Upgrade args hash: `1c08f4660179db8e87a4a5060ca93ea2f476fed7c25a2bd92f9cf8375eff87d9`

Target canister: `ghsi2-tqaaa-aaaan-aaaca-cai`

Previous Bitcoin proposal: https://dashboard.internetcomputer.org/proposal/143242

---

## Motivation
Exclude other addresses from address UTXO range queries.


## Release Notes

```
git log --format='%C(auto) %h %s' 4a7f02163277b24341f2f84cd02b1c20a22e7aeb..e21267ebd071a9f3745e04424d3293b76c96589c -- canister
e21267e chore(ic-btc-canister): release/2026-11-02 (#552)
a479aca fix: DEFI-3039: Exclude other addresses from address UTXO range queries (#551)
 ```

## Upgrade args

```
git fetch
git checkout e21267ebd071a9f3745e04424d3293b76c96589c
didc encode -d canister/candid.did -t '(canister_arg)' '(variant { upgrade })' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout e21267ebd071a9f3745e04424d3293b76c96589c
"./scripts/docker-build" "ic-btc-canister"
sha256sum ./ic-btc-canister.wasm.gz
```
