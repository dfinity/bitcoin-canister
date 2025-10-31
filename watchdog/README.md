# Watchdog canister

Watchdog service for canisters that compares their latest block height against several explorer APIs and decides if the
canister is healthy or not.

## Networks supported

- Bitcoin Mainnet
- Bitcoin Testnet
- Dogecoin Mainnet

## Commands

```sh
$ dfx stop

$ dfx start --background  --clean

$ dfx deploy watchdog

...
URLs:
  Backend canister via Candid interface:
    watchdog: http://127.0.0.1:4943/?canisterId=ryjl3-tyaaa-aaaaa-aaaba-cai&id=rrkah-fqaaa-aaaaa-aaaaq-cai
```
