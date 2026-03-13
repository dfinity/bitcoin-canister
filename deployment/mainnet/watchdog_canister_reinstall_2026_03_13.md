# Proposal to reinstall the Bitcoin watchdog canister

Repository: `https://github.com/dfinity/bitcoin-canister.git`

Git hash: `4e1ea8fd4bd2e2fb85350e4b1b3d4cc2410e389b`

New compressed Wasm hash: `541a859230e0bebf4e32573eee07755bc8a9602185a4675be60b87ed74ab1071`

Install args hash: `eb90f1eb43a54053ea23e72f1187b5d7c2182bec7d6853586c009c157f345560`

Target canister: `gatoo-6iaaa-aaaan-aaacq-cai`

Previous Bitcoin watchdog proposal: https://dashboard.internetcomputer.org/proposal/138469

---

## Motivation

Reinstall the watchdog canister monitoring the Bitcoin canister (Bitcoin mainnet network) using the latest release [watchdog/release/2026-03-13](https://github.com/dfinity/bitcoin-canister/releases/tag/watchdog%2Frelease%2F2026-03-13).

This proposal replaces the `bitcoin_network` candid service argument with a new `watchdog_arg` service argument. This new argument contains two variants: `init` which can be used to specify the monitored canister during initialization and `update` which is a null argument. This ensures that configuring the canister monitored can only be done when the watchdog canister is installed. This implies that the configuration should be stored in stable memory so that it is persisted across upgrades. For this reason, it is necessary to reinstall the canister to initialize the configuration in stable memory.

Moreover, this proposal aims to use the new `get_blockchain_info` endpoint of the Bitcoin canister to retrieve its height instead of retrieving it from the `/metrics` HTTP endpoint using HTTPs outcalls which are less reliable than intercanister calls.

The `health_status` endpoint is now deprecated and instead the `health_status_v2` endpoint should be used. This new endpoint returns providers as text which enables adding new providers without making breaking changes to the endpoint.

This proposal also updates the list of providers used by the watchdog to fetch Bitcoin mainnet latest height:
- Adds the `api_bitcore` provider (https://api.bitcore.io/api/BTC/mainnet/block?limit=1).
- Removes the `api_bitaps` provider (https://api.bitaps.com/btc/v1/blockchain/block/last): stuck for several weeks.
- Removes the `chain_api_btc_com` provider (https://chain.api.btc.com/v3/block/latest): unresponsive.
- Removes the `bitcoinexplorer.org` provider (https://bitcoinexplorer.org/api/blocks/tip): unresponsive.

Finally, this proposal simplifies the core logic of the Watchdog canister, adds support for the Dogecoin network, removes retrieving the unnecessary latest block hash from providers, and marks the `get_config` (debug) endpoint as unstable so that breaking changes can be made.

## Release Notes

```
git log --format='%C(auto) %h %s' 292b446a0ec64158eb2c68247530870ff201f274..4e1ea8fd4bd2e2fb85350e4b1b3d4cc2410e389b -- watchdog
4e1ea8f chore(watchdog): watchdog/release/2026-03-13 (#503)
5e68e4a feat(watchdog): use `get_blockchain_info` canister endpoint to retrieve monitored canister height (#484)
efe7c9d feat(watchdog): replace api_bitaps with api_bitcore provider (#500)
e7dcc11 fix(watchdog): rename `HealthStatus` field names `canister_height` and `explorer_height` (#492)
21b1571 feat(watchdog): replace tokenview explorer with psy protocol (#493)
d9ab390 fix(e2e-tests): only use pre-built wasms if present (#488)
9dcf7e6 test: use wasms from reproducible build for various e2e tests (#473)
f29c7c2 chore: update watchdog CHANGELOG.md release/2025-12-03 (#475)
9e3d701 chore(watchdog): release/2026-01-30 (#474)
383366a ci: release plz (#464)
1a5e1bb refactor(watchdog): remove retrieving latest block hash (#459)
a5ff9b2 refactor(watchdog): refactor explorer logic (#456)
1f3cf66 chore(watchdog): remove deprecated method counter (#455)
d1cced7 refactor(watchdog): remove blockexplorer.one from list of providers (#454)
9133ce5 feat(watchdog)!: add `health_status_v2` endpoint (#450)
13c6ff2 ci: add canister endpoint checks (#447)
719173a feat(watchdog): increase interval fetch and height diff for dogecoin (#449)
91b1c67 chore: bump `ic-cdk` to v0.19.0 (#446)
fbd3aa7 chore(watchdog): clean bitcoin mainnet explorers (#441)
83427b3 chore: add Bitcoin mainnet staging target for watchdog (#443)
6f80560 fix(watchdog): add cycles for http requests when in an application subnet (#439)
7186217 refactor(watchdog)!: remove the bitcoin_ prefix from candid types (#436)
2363be4 feat(watchdog): store config in stable memory (#435)
2e9d892 feat(watchdog): add dogecoin canister monitoring (#428)
b506535 chore: upgrade ic-cdk and other dependency versions (#429)
a164eed chore(watchdog): add 2025-07-02 release to CHANGELOG.md (#418)
989c202 chore: add watchdog canister changelog (#336)
455cd1c chore: add canister_id.json and docker build script (#409)
 ```

## Install args

```
git fetch
git checkout 4e1ea8fd4bd2e2fb85350e4b1b3d4cc2410e389b
didc encode -d watchdog/candid.did -t '(watchdog_arg)' '(variant { init = record { target = variant { bitcoin_mainnet } }})' | xxd -r -p | sha256sum
```

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout 4e1ea8fd4bd2e2fb85350e4b1b3d4cc2410e389b
"./scripts/docker-build" "watchdog"
sha256sum ./watchdog.wasm.gz
```