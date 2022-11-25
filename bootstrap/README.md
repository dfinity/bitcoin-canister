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
```

## 3. Download the Bitcoin state

Run `step_1`, which downloads the bitcoin state. This can several hours.

```
./step_1.sh $BITCOIN_DIR $HEIGHT
```

Once it's done, run the following:

```
./check_chaintip.sh $BITCOIN_DIR
```

Make sure that the outpoint of the above command specifies that you have a chain that has the status "active", and has a height of at least `$HEIGHT - 10`. For example, if you set the `$HEIGHT` to 10010 in the earlier steps, the height of the chain should be >= 10000. It would look something like this:

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

## 4. Compute the files needed for the canister state

```
./step_2.sh $BITCOIN_DIR $HEIGHT
```

```
./step_3.sh $BITCOIN_DIR $HEIGHT
```

```
./step_4.sh
```

This documented will be updated with future steps.
