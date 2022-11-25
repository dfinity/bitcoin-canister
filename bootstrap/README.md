Download Bitcoin Core 22.0. Mac users are recommended to download the .tar.gz version.

go install github.com/in3rsha/bitcoin-utxo-dump@5723696e694ebbfe52687f51e7fc0ce62ba43dc8

Unpack the .tar.gz file.

```
BITCOIN_DIR=/path/to/bitcoin-22.0/
HEIGHT= ... height of the state to compute
```

From inside this directory run:

```
./step_1.sh $BITCOIN_DIR $HEIGHT

./check_chaintip.sh $BITCOIN_DIR
```


Check that there is a chain with "active" that has a height no lower than "height - 10" (e.g. height 1010, height >= 1000)

```
./step_2.sh $BITCOIN_DIR $HEIGHT
./step_3.sh $BITCOIN_DIR $HEIGHT
./step_4.sh
```
