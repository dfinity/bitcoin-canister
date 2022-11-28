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
STABILITY_THRESHOLD=30
```

## 3. Download the Bitcoin state

Run `1_download_state.sh`, which downloads the bitcoin state. This can several hours.

```
./1_download_state.sh $BITCOIN_DIR $HEIGHT
```

Once it's done, run the following:

```
./check_chaintip.sh $BITCOIN_DIR
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

If the height returned here is < `$HEIGHT - 10`, then run `./1_download_state_retry.sh` for a minute or two, which downloads more Bitcoin blocks, and try again.

## 4. Compute the Bitcoin Canister's State

```
./2_compute_unstable_blocks.sh $BITCOIN_DIR $HEIGHT
./3_compute_block_headers.sh $BITCOIN_DIR $HEIGHT
./4_compute_utxo_dump.sh
./5_shuffle_utxo_dump.sh
./6_compute_canister_state.sh $HEIGHT $STABILITY_THRESHOLD
```

Once all these steps are complete, the canister's state will be available in this directory with the name `canister_state.bin`.

## 5. Compute the State's Hashes.

A canister's state is uploaded in "chunks" via ingress messages via the `uploader` canister. The hashes to provide to the `uploader` canister can be computed as follows:

```
cargo run --release --example compute_hashes -- --file ./canister_state.bin > chunk_hashes.txt
```

The hash of each chunk is saved in `chunk_hashes.txt`.
