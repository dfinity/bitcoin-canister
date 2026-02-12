## The Bitcoin Canister API

The canister IDs of the Bitcoin canisters for Bitcoin mainnet and testnet:

* `mainnet`:  [`ghsi2-tqaaa-aaaan-aaaca-cai`](https://dashboard.internetcomputer.org/canister/ghsi2-tqaaa-aaaan-aaaca-cai)
* `testnet` (specifically `testnet4`):  [`g4xu7-jiaaa-aaaan-aaaaq-cai`](https://dashboard.internetcomputer.org/canister/g4xu7-jiaaa-aaaan-aaaaq-cai)

Information about Bitcoin and the IC Bitcoin integration can be found in the [Bitcoin developer guides](https://developer.bitcoin.org/devguide/) and the [Bitcoin integration documentation](https://internetcomputer.org/docs/current/references/bitcoin-how-it-works).

The Bitcoin canister exposes the following endpoints.

### `bitcoin_get_utxos`

```
type network = variant {
  mainnet;
  testnet;  // Bitcoin testnet4.
  regtest;
};

type satoshi = nat64;

type address = text;

type block_hash = blob;

type block_height = nat32;

type outpoint = record {
  txid : blob;
  vout : nat32;
};

type utxo = record {
  outpoint : outpoint;
  value : satoshi;
  height : block_height;
};

type get_utxos_request = record {
  network : network;
  address : address;
  filter : opt variant {
    min_confirmations : nat32;
    page : blob;
  };
};

type get_utxos_response = record {
  utxos : vec utxo;
  tip_block_hash : block_hash;
  tip_height : block_height;
  next_page : opt blob;
};

bitcoin_get_utxos : (get_utxos_request) -> (get_utxos_response);
```

This endpoint can only be called by canisters, i.e., it cannot be called by external users via ingress messages.

Given a `get_utxos_request`, which must specify a Bitcoin address and a Bitcoin network (`mainnet`, `testnet`, or `regtest`), the function returns all unspent transaction outputs (UTXOs) associated with the provided address in the specified Bitcoin network based on the current view of the Bitcoin blockchain available to the Bitcoin canister. The UTXOs are returned sorted by block height in descending order.

The following address formats are supported:

-   Pay to public key hash (P2PKH)

-   Pay to script hash (P2SH)

-   Pay to witness public key hash (P2WPKH)

-   Pay to witness script hash (P2WSH)

-   Pay to taproot (P2TR)

If the address is malformed, the call is rejected.

The optional `filter` parameter can be used to restrict the set of returned UTXOs, either providing a minimum number of confirmations or a page reference when pagination is used for addresses with many UTXOs. In the first case, only UTXOs with at least the provided number of confirmations are returned, i.e., transactions with fewer than this number of confirmations are not considered. In other words, if the number of confirmations is `c`, an output is returned if it occurred in a transaction with at least `c` confirmations and there is no transaction that spends the same output with at least `c` confirmations.

There is an upper bound on the minimum number of confirmations, which varies with the difficulty target.
 If a larger minimum number of confirmations is specified, the call is rejected. Note that this is not a severe restriction as the minimum number of confirmations used in practice is around 6, which is approximately an order of magnitude lower than the upper bound under normal operation.

It is important to note that the validity of transactions is not verified in the Bitcoin canister. The Bitcoin canister relies on the proof of work that goes into the blocks and the verification of the blocks in the Bitcoin network. For a newly discovered block, a regular Bitcoin (full) node therefore provides a higher level of security than the Bitcoin canister, which implies that it is advisable to set the number of confirmations to a reasonably large value, such as 6, to gain confidence in the correctness of the returned UTXOs.

There is an upper bound of 10,000 UTXOs that can be returned in a single request. For addresses that contain sufficiently many UTXOs, a partial set of the address's UTXOs are returned along with a page reference.

In the second case, a page reference (a series of bytes) must be provided, which instructs the Bitcoin canister to collect UTXOs starting from the corresponding "page".

A `get_utxos_request` without the optional `filter` results in a request that considers the full blockchain, which is equivalent to setting `min_confirmations` to 0.

The recommended workflow is to issue a request with the desired number of confirmations. If the `next_page` field in the response is not empty, there are more UTXOs than in the returned vector. In that case, the `page` field should be set to the `next_page` bytes in the subsequent request to obtain the next batch of UTXOs.

### `bitcoin_get_utxos_query`

```
bitcoin_get_balance_query : (get_balance_request) -> (satoshi) query;
```


This endpoint is identical to `bitcoin_get_utxos` but can _only_ be invoked in a query call.
It provides a quick result, without incurring any costs in cycles, but the result may not be considered trustworthy as it comes from a single replica.

### `bitcoin_get_balance`

```
type get_balance_request = record {
  network : network;
  address : address;
  min_confirmations : opt nat32;
};

bitcoin_get_balance : (get_balance_request) -> (satoshi);
```

This endpoint can only be called by canisters, i.e., it cannot be called by external users via ingress messages.

Given a `get_balance_request`, which must specify a Bitcoin address and a Bitcoin network (`mainnet`, `testnet`, or `regtest`), the function returns the current balance of this address in `satoshi` (10^8 satoshi = 1 bitcoin) in the specified Bitcoin network. The same address formats as for `bitcoin_get_utxos` are supported.

If the address is malformed, the call is rejected.

The optional `min_confirmations` parameter can be used to limit the set of considered UTXOs for the calculation of the balance to those with at least the provided number of confirmations in the same manner as for the `bitcoin_get_utxos` call.

Given an address and the optional `min_confirmations` parameter, `bitcoin_get_balance` iterates over all UTXOs, i.e., the same balance is returned as when calling `bitcoin_get_utxos` for the same address and the same number of confirmations and, if necessary, using pagination to get all UTXOs for the same tip hash.

### `bitcoin_get_balance_query`

```
bitcoin_get_balance_query : (get_balance_request) -> (satoshi) query;
```

This endpoint is identical to `bitcoin_get_balance` but can _only_ be invoked in a query call.
It provides a quick result, without incurring any costs in cycles, but the result may not be considered trustworthy as it comes from a single replica.

### `bitcoin_get_current_fee_percentiles`

```
type get_current_fee_percentiles_request = record {
  network : network;
};

type millisatoshi_per_byte = nat64;

bitcoin_get_current_fee_percentiles : (get_current_fee_percentiles_request) -> (vec millisatoshi_per_byte);
```

This endpoint can only be called by canisters, i.e., it cannot be called by external users via ingress messages.

The transaction fees in the Bitcoin network change dynamically based on the number of pending transactions. It must be possible for a canister to determine an adequate fee when creating a Bitcoin transaction.

This function returns fee percentiles, measured in millisatoshi/vbyte (1000 millisatoshi = 1 satoshi), over the last 10,000 transactions in the specified network, i.e., over the transactions in the last approximately 4-10 blocks.

The [standard nearest-rank estimation method](https://en.wikipedia.org/wiki/Percentile#The_nearest-rank_method), inclusive, with the addition of a 0th percentile is used. Concretely, for any i from 1 to 100, the ith percentile is the fee with rank `⌈i * 100⌉`. The 0th percentile is defined as the smallest fee (excluding coinbase transactions).

### `bitcoin_get_block_headers`

```
type block_header = blob;

type get_block_headers_request = record {
  start_height : block_height;
  end_height : opt block_height;
  network : network;
};

type get_block_headers_response = record {
  tip_height : block_height;
  block_headers : vec block_header;
};

bitcoin_get_block_headers : (get_block_headers_request) -> (get_block_headers_response);
```

This endpoint can only be called by canisters, i.e., it cannot be called by external users via ingress messages.

Given a start height, an optional end height, and a Bitcoin network (`mainnet`, `testnet`, or `regtest`), the function returns the block headers in the provided range. The range is inclusive, i.e., the block headers at the start and end heights are returned as well.
An error is returned when an end height is specified that is greater than the tip height.

If no end height is specified, all blocks until the tip height, i.e., the largest available height, are returned. However, if the range from the start height to the end height or the tip height is large, only a prefix of the requested block headers may be returned in order to bound the size of the response.

The response is guaranteed to contain the block headers in order: if it contains any block headers, the first block header occurs at the start height, the second block header occurs at the start height plus one and so forth.

The response is a record consisting of the tip height and the vector of block headers.
The block headers are 80-byte blobs in the [standard Bitcoin format](https://developer.bitcoin.org/reference/block_chain.html#block-headers).

### `bitcoin_send_transaction`
```
type send_transaction_request = record {
  network : network;
  transaction : blob;
};

bitcoin_send_transaction : (send_transaction_request) -> ();
```

This endpoint can only be called by canisters, i.e., it cannot be called by external users via ingress messages.

Given a `send_transaction_request`, which must specify a `blob` of a Bitcoin transaction and a Bitcoin network (`mainnet`, `testnet`, or `regtest`), the Bitcoin canister verifies that the transaction is well formed.
If this is the case, the transaction is forwarded to the specified Bitcoin network. Note that the function does not provide any guarantees that the transaction will make it into the mempool or that the transaction will ever appear in a block.

### `get_config`
```
type flag = variant {
  enabled;
  disabled;
};

type fees = record {
  get_utxos_base : nat;
  get_utxos_cycles_per_ten_instructions : nat;
  get_utxos_maximum : nat;
  get_balance : nat;
  get_balance_maximum : nat;
  get_current_fee_percentiles : nat;
  get_current_fee_percentiles_maximum : nat;
  send_transaction_base : nat;
  send_transaction_per_byte : nat;
  get_block_headers_base : nat;
  get_block_headers_cycles_per_ten_instructions : nat;
  get_block_headers_maximum : nat;
};

type config = record {
  stability_threshold : nat;
  network : network;
  blocks_source : principal;
  syncing : flag;
  fees : fees;
  api_access : flag;
  disable_api_if_not_fully_synced : flag;
  watchdog_canister : opt principal;
  burn_cycles : flag;
  lazily_evaluate_fee_percentiles : flag;
};

get_config : () -> (config) query;
```

This endpoint returns the current configuration of the Bitcoin canister.
It specifies the following parameters:

* `stability_threshold`: This is the threshold that defines the level of "difficulty-based stability" that a Bitcoin block before it is considered stable. When a block becomes stable, its transactions are applied to the UTXO set. Subsequently, the block can be discarded to free up memory. Details about the stability mechanism can be found on the Bitcoin integration [wiki page](https://wiki.internetcomputer.org/wiki/Bitcoin_Integration) under "Fork Resolution".
* `network`: This parameter indicates whether the Bitcoin canister is connected to Bitcoin mainnet, testnet (v4), or regtest.
* `syncing`: This flag indicates whether the Bitcoin canister is actively ingesting blocks to update its state.
* `fees`: This record specifies how many cycles must be attached when invoking the individual endpoints. More information about API fees can be found in the [Bitcoin integration documentation](https://internetcomputer.org/docs/current/references/bitcoin-how-it-works#api-fees-and-pricing).
* `api_access`: This flag indicates whether access to the endpoints is enabled.
* `disable_api_if_not_fully_synced`: This flag indicates whether access to the endpoints is automatically disabled if the _watchdog canister_ indicates that the Bitcoin canister is lagging behind in the sense that its state is more than 2 blocks behind the Bitcoin blockchain. 
* `watchdog_canister`: This is the principal ID of the watchdog canister. If this canister observes that the Bitcoin canister is lagging behind, it is authorized to disable API access.
* `burn_cycles`: This flag indicates whether received cycles are burned.
* `lazily_evaluate_fee_percentiles`: This flag indicates whether fee percentiles are only evaluated when fees are requested, rather than updating them automatically whenever a newly received block is processed.

### `set_config`
```
type set_config_request = record {
  stability_threshold : opt nat;
  syncing : opt flag;
  fees : opt fees;
  api_access : opt flag;
  disable_api_if_not_fully_synced : opt flag;
  watchdog_canister : opt opt principal;
  burn_cycles : opt flag;
  lazily_evaluate_fee_percentiles : opt flag;
};

set_config : (set_config_request) -> ();
```

This endpoint is used to update the configuration. The watchdog canister can only set the API access flag. All other configuration can only be updated by the controller of the canister. For the main Bitcoin canister (connected to Bitcoin mainnet), the only controller is the NNS root canister.

### `get_blockchain_info`

```
type blockchain_info = record {
  height : block_height;
  block_hash : block_hash;
  timestamp : nat32;
  difficulty : nat;
  utxos_length : nat64;
};

get_blockchain_info : () -> (blockchain_info) query;
```

This endpoint returns information about the canister's current view of the Bitcoin blockchain, which includes the following:

* `height`: The height of the main chain tip.
* `block_hash`: The hash of the tip block as a `block_hash` (blob, 32 bytes).
* `timestamp`: The Unix timestamp of the tip block.
* `difficulty`: The difficulty of the tip block.
* `utxos_length`: The total number of UTXOs up to the main chain tip.

The main chain is defined as the longest chain of blocks with an "uncontested" tip — meaning there is no other block at
the same height as the tip.

This endpoint is primarily intended for monitoring purposes, such as by the watchdog canister. Unlike other endpoints:

* It does **not** require the API to be enabled (`api_access` flag).
* It does **not** require the canister to be fully synced.

### Byte Order

Since the Bitcoin canister provides a low-level interface, it uses the [same byte order as Bitcoin uses internally](https://learnmeabitcoin.com/technical/general/byte-order).

A quirk of Bitcoin is that the byte order is reversed for transaction and block hashes when displayed, for example, in logs and blockchain explorers.
For example, the actual block hash for the block comes out of the hash function like this:

```
5e6ed4fdff39104b0a2fea7ffc606d9644d3144a2fc2aca60fd137c66914901e
```

It is displayed the same way on [BTC canister public dashboard](https://dashboard.internetcomputer.org/canister/g4xu7-jiaaa-aaaan-aaaaq-cai) if you call `bitcoin_get_utxos_query` for `testnet` and address `tb1q6cvfmeqhl3ckgsv3d9tzxpjlgec7smd32a9a3d`.

But when searching for [this address](https://mempool.space/testnet4/address/tb1q6cvfmeqhl3ckgsv3d9tzxpjlgec7smd32a9a3d) or [this transaction](https://mempool.space/testnet4/tx/1e901469c637d10fa6acc22f4a14d344966d60fc7fea2f0a4b1039fffdd46e5e) in the mempool or on a block explorer, the following byte order is displayed:

```
1e901469c637d10fa6acc22f4a14d344966d60fc7fea2f0a4b1039fffdd46e5e
```
