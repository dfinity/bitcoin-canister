# Computing the Bitcoin State

Rather than syncing the Bitcoin canister from genesis, which can take several weeks, the state of the canister can be computed offline much more quickly with the help of `bitcoind`.

## Requirements

* A linux machine
* \>= 16GiB RAM
* \>= 100GB of disk space

## 1. Download Bitcoin Core

Download Bitcoin Core 22.0

```shell
curl -O https://bitcoin.org/bin/bitcoin-core-22.0/bitcoin-22.0-x86_64-linux-gnu.tar.gz
```

Unpack the `tar.gz` file

```shell
tar -xvf bitcoin-22.0-x86_64-linux-gnu.tar.gz
```

Install the `bitcoin-utxo-dump` utility (requires `go` lang to be installed):

```shell
go install github.com/in3rsha/bitcoin-utxo-dump@5723696e694ebbfe52687f51e7fc0ce62ba43dc8
```

## 2. Setup Environment Variables

```shell
BITCOIN_DIR=/path/to/bitcoin-22.0/
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

If the height returned here is < `$HEIGHT - 10`, then run `./1_download_state_retry.sh $BITCOIN_DIR $NETWORK` for a minute or two, which downloads more Bitcoin blocks, and try again.

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

Once all these steps are complete, the canister's state will be available in this directory with the name `canister_state.bin`.

## 5. Compute the State's Hashes.

A canister's state is uploaded in "chunks" via ingress messages via the `uploader` canister. The hashes to provide to the `uploader` canister can be computed as follows:

```shell
cargo run --release --example compute_hashes -- --file ./canister_state.bin > chunk_hashes.txt
```

The hash of each chunk is saved in `chunk_hashes.txt`.
