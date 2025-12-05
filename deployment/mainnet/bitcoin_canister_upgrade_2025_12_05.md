# Proposal to upgrade the Bitcoin canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `8d512125e772d31e2bb8692e0473fa48dc13d19e`

New compressed Wasm hash: `ee669a352fbca79f21aa6d0b7190f4daae74ef203b0ccf8d7fb82123dfd2d6a6`

Upgrade args hash: `0fee102bd16b053022b69f2c65fd5e2f41d150ce9c214ac8731cfaf496ebda4e`

Target canister: `ghsi2-tqaaa-aaaan-aaaca-cai`

Previous Bitcoin canister proposal: https://dashboard.internetcomputer.org/proposal/138813

---

## Motivation

Regular upgrade of the Bitcoin canister to the latest [release/2025-12-03](https://github.com/dfinity/bitcoin-canister/releases/tag/release%2F2025-12-03).

## Release Notes

```
git log --format='%C(auto) %h %s' 46e1a4cb276349aec9b31e400f88712502f5ad9a..8d512125e772d31e2bb8692e0473fa48dc13d19e -- canister interface types
8d51212 fix: Remove custom PartialOrd implementation for Utxo type (#451)
13c6ff2 ci: add canister endpoint checks (#447)
91b1c67 chore: bump `ic-cdk` to v0.19.0 (#446)
7f84397 fix: add `burn_cycles` field to `SetConfigRequest` (#444)
02af290 refactor: avoid panic in block tree deserialization  (#438)
f202301 refactor: generic support for BlockTree serialization (#434)
095a2b4 refactor: make BlockTree generic (#432)
e413147 refactor: use fixed size array for BlockHash type  (#433)
af000cc refactor: replace RefCell<Option<T>> with OnceCell<T> (#431)
b506535 chore: upgrade ic-cdk and other dependency versions (#429)
bd965ea chore(ic-btc-interface): release v0.2.3 (#426)
```

## Upgrade args

```
git fetch
git checkout 8d512125e772d31e2bb8692e0473fa48dc13d19e
didc encode '()' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.
NOTE: This process is not yet guaranteed to match on Apple Silicon.

```
git fetch
git checkout 8d512125e772d31e2bb8692e0473fa48dc13d19e
docker build -t canisters .
docker run --rm canisters cat /ic-btc-canister.wasm.gz > ic-btc-canister.wasm.gz
sha256sum ic-btc-canister.wasm.gz
```
