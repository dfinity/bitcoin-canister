
```shell
EFFECTIVE_CANISTER_ID="5v3p4-iyaaa-aaaaa-qaaaa-cai"; \
    TESTNET_BITCOIN_CANISTER_ID="g4xu7-jiaaa-aaaan-aaaaq-cai"; \
    TESTNET_WATCHDOG_CANISTER_ID="gjqfs-iaaaa-aaaan-aaada-cai"; \
    MAINNET_BITCOIN_CANISTER_ID="ghsi2-tqaaa-aaaan-aaaca-cai"; \
    MAINNET_WATCHDOG_CANISTER_ID="gatoo-6iaaa-aaaan-aaacq-cai"
```

When installing canister on a testnet first start a farm testnet via `$ ict testnet create`:

```shell
# In a separate terminal and in separate folder clone IC-repo
$ git clone git@github.com:dfinity/ic.git
$ cd ic

# If you are on remote machine make sure to propagate your credentials (otherwise grafana will not start)
$ ssh-add -L

# Start a container to run a testnet inside
$ ./ci/container/container-run.sh

# Before starting the testnet double check `small_bitcoin` testnet settings.
# https://github.com/dfinity/ic/blob/256c598835d637b0b58c5e2117bca011ec417a61/rs/tests/testnets/small_bitcoin.rs#L2
# Setup lifetime big enough for your experiment, provide output directory and log file
$ clear; ict testnet create small_bitcoin --lifetime-mins=10080 --output-dir=./test_tmpdir \
  > output.secret

# Same but with custom grafana dashboards
$ clear; ict testnet create small_bitcoin --lifetime-mins=10080 --output-dir=./test_tmpdir \
  --k8s-branch <repo-branch-name> \
  > output.secret
```

In the `output.secret` file find and save system subnet IPv6 and links to grafana

```shell
      {
        "nodes": [
          {
            ...
            "ipv6": "2602:xx:xx:xx:xx:xx:xx:df47" # <- YOU NEED THIS IPv6 OF SYSTEM NODE
          }
        ],
        ...
        "subnet_type": "system"
      },
  ...
  "grafana": "Grafana at http://grafana.XXX", # <- YOU NEED THIS URL
```

Update your `dfx.json` with IPv6 from the above:

```json
    "testnet": {
      "providers": [
        "http://[2602:xx:xx:xx:xx:xx:xx:df47]:8080" // <- USE IPv6 FROM THE ABOVE
      ],
      "type": "persistent"
    }
```

If you want to deploy both `testnet` and `mainnet` canisters via dfx you might want to clone their setups in `dfx.json`, so instead of having `bitcoin` you have `bitcoin_t` and `bitcoin_m`, same for `watchdog` (`watchdog_t`, `watchdog_m`).

Create corresponding canister
```shell
# (Optional) remove current canister ids. 
$ rm canister_ids.json

$ dfx canister create bitcoin_t --no-wallet \
    --network testnet \
    --subnet-type system \
    --specified-id $TESTNET_BITCOIN_CANISTER_ID \
    --provisional-create-canister-effective-canister-id $EFFECTIVE_CANISTER_ID \
    --with-cycles 1000000000000000000
```

```shell
$ dfx canister status --network testnet $TESTNET_BITCOIN_CANISTER_ID
```

Upgrade bitcoin canister
```shell
$ dfx canister stop --network testnet $TESTNET_BITCOIN_CANISTER_ID

$ dfx canister install \
    --network testnet $TESTNET_BITCOIN_CANISTER_ID \
    --mode upgrade \
    --wasm ./ic-btc-canister.wasm.gz \
    --argument "$ARG"

$ dfx canister start --network testnet $TESTNET_BITCOIN_CANISTER_ID
```