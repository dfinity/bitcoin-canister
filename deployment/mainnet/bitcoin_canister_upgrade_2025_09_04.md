# Proposal to upgrade the Bitcoin canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `292b446a0ec64158eb2c68247530870ff201f274`

New compressed Wasm hash: `42af59ae4fd5041f30f8ac12f324e3a93533de0cb89cd2278100c2389cbfff65`

Upgrade args hash: `0fee102bd16b053022b69f2c65fd5e2f41d150ce9c214ac8731cfaf496ebda4e`

Target canister: `ghsi2-tqaaa-aaaan-aaaca-cai`

Previous Bitcoin canister proposal: https://dashboard.internetcomputer.org/proposal/132220

---

## Motivation

This proposal aims to upgrade the Bitcoin canister to incorporate several bug fixes, add metrics for better monitoring,
and upgrade dependencies to their latest versions.

## Release Notes

```
git log --format='%C(auto) %h %s' release/2024-08-30.. -- canister
 b86ac5f fix: fix stuck canister after upgrade during block fetch (#405)
 a611575 feat(runtime): add mocking time (#401)
 6167314 fix: adaptive max depth limit calculation for unstable blocks tree (#385)
 ee3d395 fix: [EXC-1987] Fix encoding of get_block_headers metrics on Bitcoin canister (#383)
 82863a8 chore: adjust testnet unstable max depth difference (#382)
 b2177ec chore: cleanups (#381)
 063beb8 fix: fix unstable tree block stability check for testnet (#379)
 ced4b1b fix: fix memory leak (#378)
 e19928c feat: add default fees for mainnet/testnet networks (#376)
 cbf67c8 chore: update dfx to 0.23 and rust to 1.81 (#372)
 f96124b chore: update get_successors metrics (#364)
 0e9a055 fix: get_successors request sends unique hashes (#363)
 f63b04f feat: add get_successors_request_interval histogram metric (#361)
 414a7fa feat: add get_successors request / response metrics for bitcoin canister (#360)
 6f88899 fix: reduce Bitcoin canister logs by skipping full GetSuccessorsResponse (#359)
 93213b2 chore: add instructions for generating testnet_blocks.txt (#351)
 47c5d1f feat: upgrade bitcoin crate to 0.32.4 for testnet4 support (#349)
 64f3183 fix: remove `rand` dependency from Bitcoin canister (#348)
 f7afb9d chore: Upgrade stable structures to 0.6.7 (#346)
 32c70fa chore: rename usage of BlockHeader to Header for bitcoin crate v.0.32.4 update (#345)
 ```

## Upgrade args

```
git fetch
git checkout 292b446a0ec64158eb2c68247530870ff201f274
didc encode '()' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout 292b446a0ec64158eb2c68247530870ff201f274
docker build -t canisters .
docker run --rm canisters cat /ic-btc-canister.wasm.gz > ic-btc-canister.wasm.gz
sha256sum ic-btc-canister.wasm.gz
```