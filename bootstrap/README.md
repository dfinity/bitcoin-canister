# Computing the Bitcoin State

Rather than syncing the Bitcoin canister from genesis, which can take several weeks, the state of the canister can be computed offline much more quickly with the help of `bitcoind`.

## Requirements

* A linux machine
* \>= 16GiB RAM
* \>= 100GB of disk space

## 1. Download Bitcoin Core

Download [Bitcoin Core 22.0](https://bitcoin.org/bin/bitcoin-core-22.0/bitcoin-22.0-x86_64-linux-gnu.tar.gz) and unpack the `tar.gz` file.

Install the `bitcoin-utxo-dump` utility (requires `go` lang to be installed):

```
go install github.com/in3rsha/bitcoin-utxo-dump@5723696e694ebbfe52687f51e7fc0ce62ba43dc8
```

## 2. Setup Environment Variables

```
BITCOIN_DIR=/path/to/bitcoin-22.0/
HEIGHT=<height of the state you want to compute>
STABILITY_THRESHOLD=<desired stability threshold>
NETWORK=<mainnet or testnet>
```

## 3. Checkout the working revision
See [Proposal](https://dashboard.internetcomputer.org/proposal/94253)

```
git checkout fdfced51d002d7f16908642f29216a869ecd3627
```

## 4. Download the Bitcoin state

Run `1_download_state.sh`, which downloads the bitcoin state. This can several hours.

```
./1_download_state.sh $BITCOIN_DIR $HEIGHT $NETWORK
```

Once it's done, run the following:

```
./check_chaintip.sh $BITCOIN_DIR $NETWORK
```

Make sure that the output of the above command specifies that you have a chain that has the status "active", and has a height of at least `$HEIGHT - 10`. For example, if you set the `$HEIGHT` to 10010 in the earlier steps, the height of the chain should be >= 10000. It should look something like this:

```
[
  {
    "height": <height>,
    "hash": "<block hash>",
    "branchlen": 0,
    "status": "active"
  }
]
```

If the height returned here is < `$HEIGHT - 10`, then run `./1_download_state_retry.sh $BITCOIN_DIR $NETWORK` for a minute or two, which downloads more Bitcoin blocks, and try again.

## 5. Compute the Bitcoin Canister's State

```
./2_compute_unstable_blocks.sh $BITCOIN_DIR $HEIGHT $NETWORK
./3_compute_block_headers.sh $BITCOIN_DIR $HEIGHT $NETWORK
./4_compute_utxo_dump.sh $NETWORK
./5_shuffle_utxo_dump.sh
./6_compute_canister_state.sh $HEIGHT $STABILITY_THRESHOLD $NETWORK
```

Once all these steps are complete, the canister's state will be available in this directory with the name `canister_state.bin`.

## 6. Compute the State's Hashes

A canister's state is uploaded in "chunks" via ingress messages via the `uploader` canister. The hashes to provide to the `uploader` canister can be computed as follows:

```
cargo run --release --example compute_hashes -- --file ./canister_state.bin > chunk_hashes.txt
```

The hash of each chunk is saved in `chunk_hashes.txt`.

## 7. Hardcode the caches into the uploader canister

```
cp bootstrap/chunk_hashes.txt bootstrap/uploader/src/chunk_hashes.txt
```

Update the `state_size` in `bootstrap/uploader/src/main.rs` to be equal to the state size in 64k blocks, i.e. `echo $(( $(stat -c %s bootstrap/canister_state.bin) / 65536 ))`

## 8. Deploy the uploader canister

`dfx` version `0.12.1` is known to work

Add desired network to the `dfx.json` and set `NETWORK` to match it.

Create, build and install:

```
dfx canister --network=$NETWORK create uploader --no-wallet
dfx build $uploader --network=$NETWORK
dfx canister install uploader --network=$NETWORK
```

## 9. Upload the state

Set `CANISTER` to match canister id, `IP` to match one of the nodes.

```
cd uploader
cargo run --example upload -- --canister-id $CANISTER --state ../canister_state.bin --ic-network http://[${IP}]:8080  --fetch-root-key
```

If the desired node gets unhealthy, the uploader crashes but it continues to upload on restart. Takes few hours to few days.

## 10. Hardcode the canister id into the replica binary

Set the bitcoin canister id(s) in `rs/config/src/execution_environment.rs` to match the uploader canister id. Build image and upgrade the subnet.

## 11. Substitute uploader canister binary with bitcoin canister

Edit `canister_ids.json`: rename "uploader" to "bitcoin"

Upgrade binary, e.g.

```
dfx build bitcoin --network=$NETWORK
dfx canister --network $NETWORK install bitcoin --argument '(record {network = (variant {mainnet}); api_access = (variant {enabled}); blocks_source = (principal "aaaaa-aa"); fees = (record { get_current_fee_percentiles = 0; get_utxos_maximum = 0; get_current_fee_percentiles_maximum = 0; send_transaction_per_byte = 0; get_balance = 0; get_utxos_cycles_per_ten_instructions = 0; get_utxos_base = 0; get_balance_maximum = 0; send_transaction_base = 0}); stability_threshold = 100; syncing = (variant {enabled})})' --mode=upgrade
```

## 12. Check if the bitcoin canister is syncing

```
dfx canister call bitcoin --network=$NETWORK http_request '(record { url = "/metrics"; body = vec{0:nat8}; headers = vec{}; method = "GET" })' | sed 's/\\0a#/\n/g' | sed 's/\\0a/\t/g ' | grep chain_height
```

The chain height should increase after the bitcoin canister manages to connect to the bitcoin network, which can take couple of minutes.
