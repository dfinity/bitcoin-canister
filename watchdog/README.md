# bitcoin-canister-watchdog

Watchdog service for a bitcoin_canister that compares its latest block height against some bitcoin explorer APIs and decides if bitcoin_canister is healthy or not.

Watchdog collects data via HTTP out calls with `timer_interval_secs` interval, it expects to get the data from at least `min_explorers` explorers. The status is `ok` if bitcoin_canister latest block height is within `blocks_behind_threshold` and `blocks_ahead_threshold` difference from a calculated pivot block height (currently a median from all the available explorers).

## Commands

```sh
$ dfx stop

$ dfx start --background  --clean

$ dfx deploy watchdog '(0)'

...
URLs:
  Backend canister via Candid interface:
    watchdog: http://127.0.0.1:4943/?canisterId=ryjl3-tyaaa-aaaaa-aaaba-cai&id=rrkah-fqaaa-aaaaa-aaaaq-cai
```

## API

Actual health report:

```json
{
    "config": {
        "timer_interval_secs": 60,
        "min_explorers": 2,
        "blocks_ahead_threshold": 2,
        "blocks_behind_threshold": -2,
        "storage_ttl_millis": 300000
    },
    "status": {
        "code": "ok",
        "message": "Bitcoin canister block height is within the limits",
        "height_diff": 0
    },
    "bitcoin_canister": 783338,
    "pivot": [
        "blockchain.info",
        783338
    ],
    "explorers_n": 4,
    "explorers": [
        [
            "api.blockcypher.com",
            783338
        ],
        [
            "blockchain.info",
            783338
        ],
        [
            "blockstream.info",
            783338
        ],
        [
            "chain.api.btc.com",
            783338
        ]
    ]
}
```
