
## Deploy and re-deploy the canister from start

```bash
$ EFFECTIVE_CANISTER_ID="5v3p4-iyaaa-aaaaa-qaaaa-cai"; \
    TESTNET_BITCOIN_CANISTER_ID="g4xu7-jiaaa-aaaan-aaaaq-cai"; \
    TESTNET_WATCHDOG_CANISTER_ID="gjqfs-iaaaa-aaaan-aaada-cai"; \
    MAINNET_BITCOIN_CANISTER_ID="ghsi2-tqaaa-aaaan-aaaca-cai"; \
    MAINNET_WATCHDOG_CANISTER_ID="gatoo-6iaaa-aaaan-aaacq-cai"

$ rm canister_ids.json
$ dfx canister create bitcoin_t --no-wallet \
    --network testnet \
    --subnet-type system \
    --specified-id $TESTNET_BITCOIN_CANISTER_ID \
    --provisional-create-canister-effective-canister-id $EFFECTIVE_CANISTER_ID \
    --with-cycles 1000000000000000000

# Start polling logs.
$ ./poll_logs.sh > canister.log

# Deploy first time.
$ dfx deploy --network testnet bitcoin_t --argument "(record {
  stability_threshold = opt 144;
  network = opt variant { testnet };
  syncing = opt variant { enabled };
  api_access = opt variant { disabled };
  watchdog_canister = opt opt principal \"$TESTNET_WATCHDOG_CANISTER_ID\";
})"

# Re-deploy.
$ dfx canister stop --network testnet $TESTNET_BITCOIN_CANISTER_ID

$ dfx deploy --network testnet bitcoin_t --mode reinstall --argument "(record {
  stability_threshold = opt 144;
  network = opt variant { testnet };
  syncing = opt variant { enabled };
  api_access = opt variant { disabled };
  watchdog_canister = opt opt principal \"$TESTNET_WATCHDOG_CANISTER_ID\";
})"

# Re-start polling logs.
$ ./poll_logs.sh > canister.log

$ dfx canister start --network testnet $TESTNET_BITCOIN_CANISTER_ID
```

## Pre-calculate the state

See [./bootstrap/README.md]

```bash
BITCOIN_DIR=./bitcoin-28.0; \
NETWORK=testnet; \
HEIGHT=65700; \
STABILITY_THRESHOLD=144
```
