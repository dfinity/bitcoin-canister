# Computing the Bitcoin State

Rather than syncing the Bitcoin canister from genesis, which can take several weeks, the state of the canister can be computed offline much more quickly with the help of `bitcoind`.

## Requirements

* A linux machine
* \>= 16GiB RAM
* \>= 100GB of disk space

## 1. Download Bitcoin Core

Go to `bootstrap` directory:

```shell
cd ./bootstrap
```

Download Bitcoin Core 28.0

```shell
curl -O https://bitcoincore.org/bin/bitcoin-core-28.0/bitcoin-28.0-x86_64-linux-gnu.tar.gz
```

Unpack the `tar.gz` file

```shell
tar -xvf bitcoin-28.0-x86_64-linux-gnu.tar.gz
```

Install the `bitcoin-utxo-dump` utility (requires `go` lang to be installed):

```shell
go install github.com/in3rsha/bitcoin-utxo-dump@5723696e694ebbfe52687f51e7fc0ce62ba43dc8
```

## 2. Setup Environment Variables

```shell
BITCOIN_DIR=./bitcoin-28.0
NETWORK=<mainnet or testnet>
HEIGHT=<height of the state you want to compute>
STABILITY_THRESHOLD=<desired stability threshold>
```

## 3. Download the Bitcoin state

Run `1_download_state.sh`, which downloads the bitcoin state. This can several hours.

```shell
./1_download_state.sh $BITCOIN_DIR $NETWORK $HEIGHT
```

Once it's done, run the following:

```shell
./check_chaintip.sh $BITCOIN_DIR $NETWORK
```

Make sure that the output of the above command specifies that you have a chain that has the status "active", and has a height of at least `$HEIGHT - 10`. For example, if you set the `$HEIGHT` to 10010 in the earlier steps, the height of the chain should be >= 10000. It should look something like this:

```shell
[
  {
    "height": <height>,
    "hash": "<block hash>",
    "branchlen": 0,
    "status": "active"
  }
]
```

If the height returned here is < `$HEIGHT - 10`, then run

```shell
./1_download_state_retry.sh $BITCOIN_DIR $NETWORK
```

for a minute or two, which downloads more Bitcoin blocks, and try again.

## 4. Compute the Bitcoin Canister's State

```shell
./2_compute_unstable_blocks.sh $BITCOIN_DIR $NETWORK $HEIGHT
```

```shell
./3_compute_block_headers.sh $BITCOIN_DIR $NETWORK $HEIGHT
```

```shell
./4_compute_utxo_dump.sh $NETWORK
```

```shell
./5_shuffle_utxo_dump.sh
```

```shell
./6_compute_canister_state.sh $NETWORK $HEIGHT $STABILITY_THRESHOLD
```

(Optional) check output data size:
```shell
$ du -sh ./output/*
13M     ./output/block_headers
1.1G    ./output/canister_state
1.1G    ./output/canister_state.bin
2.4G    ./output/data
2.4G    ./output/data_bk
120K    ./output/unstable_blocks
469M    ./output/utxodump.csv
469M    ./output/utxodump_shuffled.csv
```

Once all these steps are complete, the canister's state will be available in this directory with the name `canister_state.bin`.

## 5. Compute the State Hashes

A canister's state is uploaded in "chunks" through ingress messages to the `uploader` canister. 
The required chunk hashes can be computed as follows:

```shell
cargo run --release --example compute_hashes -- --file ./output/canister_state.bin > chunk_hashes.txt
```

The hashes of each chunk are saved in `./bootstrap/chunk_hashes.txt` and can be used later when building the `uploader` canister in Docker.

## 6. Build Canisters

```shell
# Go back to root repo directory
$ cd ..

# Build all, specifying the path to chunk_hashes.txt
$ docker build --build-arg CHUNK_HASHES_PATH=/bootstrap/chunk_hashes.txt  -t canisters .

# Extract canister's WASM
$ docker run --rm --entrypoint cat canisters /uploader.wasm.gz > uploader.wasm.gz
$ docker run --rm --entrypoint cat canisters /ic-btc-canister.wasm.gz > ic-btc-canister.wasm.gz

# Verify SHA-256 of the canister's WASM.
$ sha256sum *.wasm.gz
c6abf3605cd33d0d640a648ecc1aaf33999032775436481485468a75024f38bc  ic-btc-canister.wasm.gz
2f9a1f7ee91ce2e2c29cc78040197b2687c25ac7fd76a609c79a72c67e3ca1d8  uploader.wasm.gz
```

## 7. Install Uploader Canister & Upload Chunks

Prepare install arguments
```shell
# Get canister state size
$ ls -al ./bootstrap/output/canister_state.bin
-rw-rw-r-- 1 maksym maksym 1149304832 Jan 13 11:46 ./bootstrap/output/canister_state.bin
```

Calculate required number of pages, page is `64 * 1024` bytes
```txt
1149304832 / (64 * 1024) = 17537
```

Calculate args hash
```shell
$ didc encode -t '(nat64)' "(17537)" | xxd -r -p | sha256sum
e299fbe18558a3646ab33e5d28eec04e474339f235cf4f22dd452c98f831a249  -
```

```shell
TESTNET_BITCOIN_CANISTER_ID="g4xu7-jiaaa-aaaan-aaaaq-cai"; \
    TESTNET_WATCHDOG_CANISTER_ID="gjqfs-iaaaa-aaaan-aaada-cai"; \
    MAINNET_BITCOIN_CANISTER_ID="ghsi2-tqaaa-aaaan-aaaca-cai"; \
    MAINNET_WATCHDOG_CANISTER_ID="gatoo-6iaaa-aaaan-aaacq-cai"
```

When installing canister on a testnet first start a testnet:

```shell
# In a separate terminal and in separate folder clone IC-repo
$ git clone git@github.com:dfinity/ic.git
$ cd ic

# If you are on remote machine make sure to propagate your credentials 
# (otherwise grafana will not start)
$ ssh-add -L

# Start a container and create a `small_high_perf` testnet inside it
$ ./ci/container/container-run.sh
$ clear; ict testnet create small_high_perf --lifetime-mins=10080 --output-dir=./test_tmpdir > output.secret
```

In the `output.secret` file find and save system subnet IPv6 and links to grafana

```shell
      {
        "nodes": [
          {
            "domain": null,
            "id": "zvqrm-xxxxxx-zae",
            "ipv6": "2602:xx:xx:xx:xx:xx:xx:df47"
          }
        ],
        "subnet_id": "sj5e7-xxxx-eae",
        "subnet_type": "system"
      },

  "bn_aaaa_records": {
    "aaaa_records": [
      "2602:xx:xx:xx:xx:xx:xx:4dd4"
    ],
    "url": "XXX"
  },
  "prometheus": "Prometheus Web UI at http://prometheus.XXX",
  "grafana": "Grafana at http://grafana.XXX",

```

Update your `dfx.json` with IPv6 from the above:

```json
    "testnet": {
      "providers": [
        "http://[2602:xx:xx:xx:xx:xx:xx:df47]:8080"
      ],
      "type": "persistent"
    }
```

Install uploader canister
```shell
$ dfx canister install \
    --network testnet $TESTNET_BITCOIN_CANISTER_ID \
    --mode reinstall \
    --wasm ./uploader.wasm.gz \
    --argument "(17537 : nat64)"
```

```shell
$ dfx canister status --network testnet $TESTNET_BITCOIN_CANISTER_ID
```

Upload chunks
```shell
$ cargo run --example upload -- \
    --canister-id $TESTNET_BITCOIN_CANISTER_ID \
    --state ./bootstrap/output/canister_state.bin \
    --ic-network http://\[2602:xx:xx:xx:xx:xx:xx:df47\]:8080 \
    --fetch-root-key
```

## 8. Upgrade Bitcoin Canister

Prepare upgrade arguments
```shell
$ ARG="(opt record {
    stability_threshold = opt $STABILITY_THRESHOLD;
    syncing = opt variant { enabled };
    burn_cycles = opt variant { enabled };
    api_access = opt variant { enabled };
    lazily_evaluate_fee_percentiles = opt variant { enabled };
    fees = opt record {
        get_current_fee_percentiles = 4_000_000 : nat;
        get_block_headers_base = 20_000_000;
        get_block_headers_cycles_per_ten_instructions = 4;
        get_block_headers_maximum = 4_000_000_000;
        get_utxos_maximum = 4_000_000_000 : nat;
        get_current_fee_percentiles_maximum = 40_000_000 : nat;
        send_transaction_per_byte = 8_000_000 : nat;
        get_balance = 4_000_000 : nat;
        get_utxos_cycles_per_ten_instructions = 4 : nat;
        get_utxos_base = 20_000_000 : nat;
        get_balance_maximum = 40_000_000 : nat;
        send_transaction_base = 2_000_000_000 : nat;
    };
    watchdog_canister = opt opt principal \"$TESTNET_WATCHDOG_CANISTER_ID\";
})"
```

```shell
$ didc encode -d ./canister/candid.did -t '(opt set_config_request)' "$ARG" | xxd -r -p | sha256sum
e129040f023b1b39c3016d604366cea83180c51ec0324426fee00f27ee731f89
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
