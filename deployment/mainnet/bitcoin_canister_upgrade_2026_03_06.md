# Proposal to upgrade the Bitcoin canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `45b45d816e3f2ae6aefbffc1e5ed6e8e9b51a854`

New compressed Wasm hash: `a0dfef8f5c52e628ce3a197277f0d5fc427f3b34f6f4deb771572855616c9749`

Upgrade args hash: `1c08f4660179db8e87a4a5060ca93ea2f476fed7c25a2bd92f9cf8375eff87d9`

Target canister: `ghsi2-tqaaa-aaaan-aaaca-cai`

Previous Bitcoin canister proposal: https://dashboard.internetcomputer.org/proposal/139663

---

## Motivation

This upgrade adds the `get_blockchain_info` endpoint to the Bitcoin canister. This endpoint allows caller to query the canister for blockchain information, which includes the latest height, the hash, the timestamp and the difficulty the latest block. This endpoint can be called even if the API of the canister is disabled or if the canister is not in sync with the adapter. Later, the watchdog canister will use this endpoint to check the health of the Bitcoin canister instead of getting the latest height from the /metrics endpoint through HTTPs outcalls.

The canister's main chain selection used by the `bitcoin_get_balance`, `bitcoin_get_utxos`, `bitcoin_get_current_fee_percentiles`, and `bitcoin_get_block_headers` endpoints previously relied on the longest chain by block count. This does not match Bitcoin's consensus rule, which defines the main chain as the one with the most accumulated proof-of-work. In practice, on the Bitcoin mainnet, difficulty adjustments are gradual and bounded, so the chain with the most work will also typically be the longest. For correctness and full consistency with Bitcoin, this upgrade changes the main chain selection rule so that the chain with the greatest accumulated proof-of-work is considered the main chain. Note that this upgrade does not affect when blocks are considered stable, as block stability is calculated elsewhere and already relied on the accumulated proof-of-work.

Additionally, this upgrade adds network validation for addresses in `bitcoin_get_balance` and `bitcoin_get_utxos` requests so that an error is returned to the user if the address is for a different network (e.g. regtest address for the mainnet canister).

Finally, this upgrade adds unified canister arguments with init and upgrade variants to ease construction of the init and upgrade arguments of the canister.

## Release Notes

```
git log --format='%C(auto) %h %s' 8d512125e772d31e2bb8692e0473fa48dc13d19e..45b45d816e3f2ae6aefbffc1e5ed6e8e9b51a854 -- canister
c3c089f refactor: move CanisterArg to ic-btc-interface (#495)
6c6b712 feat: add most accumulated difficulty criterion in main chain selection (#490)
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
git checkout 45b45d816e3f2ae6aefbffc1e5ed6e8e9b51a854
didc encode -d canister/candid.did -t '(canister_arg)' '(variant { upgrade })' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout 45b45d816e3f2ae6aefbffc1e5ed6e8e9b51a854
"./scripts/docker-build" "ic-btc-canister"
sha256sum ./ic-btc-canister.wasm.gz
```