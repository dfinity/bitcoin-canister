# How to Generate `testnet_blocks.txt`

To generate the `testnet_blocks.txt` file from `./canister/test-data/testnet4_10k_blocks.dat`:

1. Set `SAVE_CHAIN_AS_HEX_FILE = true` in `./canister/src/tests.rs`.
2. Run the `testnet_10k_blocks` test from the `ic-btc-canister` package.

Run the following command:

```shell
cargo test --release -p ic-btc-canister testnet_10k_blocks
```
