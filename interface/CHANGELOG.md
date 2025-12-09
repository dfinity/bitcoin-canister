# ic-btc-interface changelog

# 0.3.0

- Removes custom PartialOrd implementation for type Utxo. This is a breaking change in terms of the semantics.
- Adds a burn_cycles field to type UtxosFilter.

# 0.2.3

- Adds two APIs: `Fees::testnet` and `Fees::mainnet`.
