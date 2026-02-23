# Proposal to upgrade the Bitcoin canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `bfa7889d6931785e0d08ffe3f8b86db6bd621b26`

New compressed Wasm hash: `b7829e6ae6100c789bb967bc9ba49c72d12c6bdf56a28c0188d688a22d75ef47`

Upgrade args hash: `1c08f4660179db8e87a4a5060ca93ea2f476fed7c25a2bd92f9cf8375eff87d9`

Target canister: `ghsi2-tqaaa-aaaan-aaaca-cai`

Previous Bitcoin canister proposal: https://dashboard.internetcomputer.org/proposal/139663

---

## Motivation

This upgrade adds the `get_blockchain_info` endpoint to the Bitcoin canister. This endpoint allows caller to query the canister for blockchain information, which includes the latest height, the hash, the timestamp and the difficulty the latest block. This endpoint can be called even if the API of the canister is disabled or if the canister is not in sync with the adapter. Later, the watchdog canister will use this endpoint to check the health of the Bitcoin canister instead of getting the latest height from the /metrics endpoint through HTTPs outcalls.

Additionally, this upgrade adds network validation for addresses in `get_balance` and `get_utxos` requests so that an error is returned to the user if the address is for a different network (e.g. regtest address for the mainnet canister).

Finally, this upgrade adds unified canister arguments with init and upgrade variants to ease construction of the init and upgrade arguments of the canister.


## Release Notes

```
git log --format='%C(auto) %h %s' 8d512125e772d31e2bb8692e0473fa48dc13d19e..bfa7889d6931785e0d08ffe3f8b86db6bd621b26 -- canister
bfa7889 chore(ic-btc-canister): release/2026-02-18 (#485)
6829527 feat: add `get_blockchain_info` endpoint (#483)
383366a ci: release plz (#464)
4ac0cc4 chore: add bitcoin canister changelog (#463)
3a3f682 feat: add network validation for addresses in get_balance and get_utxos requests (#458)
a47b80e refactor: add canister_arg to initialize and upgrade canister (#457)
 ```

## Upgrade args

```
git fetch
git checkout bfa7889d6931785e0d08ffe3f8b86db6bd621b26
didc encode -d canister/candid.did -t '(canister_arg)' '(variant { upgrade })' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout bfa7889d6931785e0d08ffe3f8b86db6bd621b26
"./scripts/docker-build" "ic-btc-canister"
sha256sum ./ic-btc-canister.wasm.gz
```