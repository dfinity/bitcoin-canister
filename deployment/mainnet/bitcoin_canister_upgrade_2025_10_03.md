# Proposal to upgrade the Bitcoin canister canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `46e1a4cb276349aec9b31e400f88712502f5ad9a`

New compressed Wasm hash: `46162c5027eb8096be14920ba4283d6fa75bf82f1221ebae9127358ef365038b`

Upgrade args hash: `0fee102bd16b053022b69f2c65fd5e2f41d150ce9c214ac8731cfaf496ebda4e`

Target canister: `ghsi2-tqaaa-aaaan-aaaca-cai`

Previous Bitcoin canister proposal: https://dashboard.internetcomputer.org/proposal/138384

---

## Motivation
TODO: THIS MUST BE FILLED OUT


## Release Notes

```
git log --format='%C(auto) %h %s' 292b446a0ec64158eb2c68247530870ff201f274..46e1a4cb276349aec9b31e400f88712502f5ad9a -- canister
46e1a4c fix: Prevent insertion of duplicate unstable blocks (#422)
2262b77 refactor: validation of bitcoin headers and blocks (#419)
98c0666 chore: remove profiling script (#414)
efb8efb refactor(types): remove useless BitcoinAddress::from_str safety check (#408)
 ```

## Upgrade args

```
git fetch
git checkout 46e1a4cb276349aec9b31e400f88712502f5ad9a
didc encode '()' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout 46e1a4cb276349aec9b31e400f88712502f5ad9a
"./scripts/docker-build" "ic-btc-canister"
sha256sum ./ic-btc-canister.wasm.gz
```