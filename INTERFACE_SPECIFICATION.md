## The Bitcoin Canister API

The canister ID of the Bitcoin canister for Bitcoin mainnet is `ghsi2-tqaaa-aaaan-aaaca-cai`.
The canister ID of the Bitcoin canister for Bitcoin testnet (v4) is `g4xu7-jiaaa-aaaan-aaaaq-cai`.

Information about Bitcoin and the IC Bitcoin integration can be found in the [Bitcoin developer guides](https://developer.bitcoin.org/devguide/) and    the [Bitcoin integration documentation](https://internetcomputer.org/docs/current/references/bitcoin-how-it-works).

### `bitcoin_get_utxos`

This endpoint can only be called by cani    sters, i.e., it cannot be called by external users via ingress messages.

Given a `get_utxos_request`, which must specify a Bitcoin address and a Bitcoin network (`mainnet` or `testnet`), the function returns all unspent transaction outputs (UTXOs) associated with the provided address in the specified Bitcoin network based on the current view of the Bitcoin blockchain available to the Bitcoin component. The UTXOs are returned sorted by block height in descending order.

The following address formats are supported:

-   Pay to public key hash (P2PKH)

-   Pay to script hash (P2SH)

-   Pay to witness public key hash (P2WPKH)

-   Pay to witness script hash (P2WSH)

-   Pay to taproot (P2TR)

If the address is malformed, the call is rejected.

The optional `filter` parameter can be used to restrict the set of returned UTXOs, either providing a minimum number of confirmations or a page reference when pagination is used for addresses with many UTXOs. In the first case, only UTXOs with at least the provided number of confirmations are returned, i.e., transactions with fewer than this number of confirmations are not considered. In other words, if the number of confirmations is `c`, an output is returned if it occurred in a transaction with at least `c` confirmations and there is no transaction that spends the same output with at least `c` confirmations.

There is an upper bound of 144 on the minimum number of confirmations. If a larger minimum number of confirmations is specified, the call is rejected. Note that this is not a severe restriction as the minimum number of confirmations is typically set to a value around 6 in practice.

It is important to note that the validity of transactions is not verified in the Bitcoin component. The Bitcoin component relies on the proof of work that goes into the blocks and the verification of the blocks in the Bitcoin network. For a newly discovered block, a regular Bitcoin (full) node therefore provides a higher level of security than the Bitcoin component, which implies that it is advisable to set the number of confirmations to a reasonably large value, such as 6, to gain confidence in the correctness of the returned UTXOs.

There is an upper bound of 10,000 UTXOs that can be returned in a single request. For addresses that contain sufficiently many UTXOs, a partial set of the address's UTXOs are returned along with a page reference.

In the second case, a page reference (a series of bytes) must be provided, which instructs the Bitcoin component to collect UTXOs starting from the corresponding "page".

A `get_utxos_request` without the optional `filter` results in a request that considers the full blockchain, which is equivalent to setting `min_confirmations` to 0.

The recommended workflow is to issue a request with the desired number of confirmations. If the `next_page` field in the response is not empty, there are more UTXOs than in the returned vector. In that case, the `page` field should be set to the `next_page` bytes in the subsequent request to obtain the next batch of UTXOs.

### `bitcoin_get_utxos_query`

This endpoint is identical to `bitcoin_get_utxos` but can _only_ be invoked in a query call.
It provides a quick result, without incurring any costs in cycles, but the result may not be considered trustworthy as it comes from a single replica.

### `bitcoin_get_balance`

This endpoint can only be called by canisters, i.e., it cannot be called by external users via ingress messages.

Given a `get_balance_request`, which must specify a Bitcoin address and a Bitcoin network (`mainnet` or `testnet`), the function returns the current balance of this address in `Satoshi` (10^8 Satoshi = 1 Bitcoin) in the specified Bitcoin network. The same address formats as for `bitcoin_get_utxos` are supported.

If the address is malformed, the call is rejected.

The optional `min_confirmations` parameter can be used to limit the set of considered UTXOs for the calculation of the balance to those with at least the provided number of confirmations in the same manner as for the `bitcoin_get_utxos` call.

Given an address and the optional `min_confirmations` parameter, `bitcoin_get_balance` iterates over all UTXOs, i.e., the same balance is returned as when calling `bitcoin_get_utxos` for the same address and the same number of confirmations and, if necessary, using pagination to get all UTXOs for the same tip hash.

### `bitcoin_get_balance_query`

This endpoint is identical to `bitcoin_get_balance` but can _only_ be invoked in a query call.
It provides a quick result, without incurring any costs in cycles, but the result may not be considered trustworthy as it comes from a single replica.

### `bitcoin_get_current_fee_percentiles`

This endpoint can only be called by canisters, i.e., it cannot be called by external users via ingress messages.

The transaction fees in the Bitcoin network change dynamically based on the number of pending transactions. It must be possible for a canister to determine an adequate fee when creating a Bitcoin transaction.

This function returns fee percentiles, measured in millisatoshi/vbyte (1000 millisatoshi = 1 satoshi), over the last 10,000 transactions in the specified network, i.e., over the transactions in the last approximately 4-10 blocks.

The [standard nearest-rank estimation method](https://en.wikipedia.org/wiki/Percentile#The_nearest-rank_method), inclusive, with the addition of a 0th percentile is used. Concretely, for any i from 1 to 100, the ith percentile is the fee with rank `⌈i * 100⌉`. The 0th percentile is defined as the smallest fee (excluding coinbase transactions).

### `bitcoin_get_block_headers`

This endpoint can only be called by canisters, i.e., it cannot be called by external users via ingress messages.

Given a start height, an optional end height, and a Bitcoin network (`mainnet` or `testnet`), the function returns the block headers in the provided range. The range is inclusive, i.e., the block headers at the start and end heights are returned as well.
An error is returned when an end height is specified that is greater than the tip height.

If no end height is specified, all blocks until the tip height, i.e., the largest available height, are returned. However, if the range from the start height to the end height or the tip height is large, only a prefix of the requested block headers may be returned in order to bound the size of the response.

The response is guaranteed to contain the block headers in order: if it contains any block headers, the first block header occurs at the start height, the second block header occurs at the start height plus one and so forth.

The response is a record consisting of the tip height and the vector of block headers.
The block headers are 80-byte blobs in the [standard Bitcoin format](https://developer.bitcoin.org/reference/block_chain.html#block-headers).

### `bitcoin_send_transaction`

This endpoint can only be called by canisters, i.e., it cannot be called by external users via ingress messages.

Given a `send_transaction_request`, which must specify a `blob` of a Bitcoin transaction and a Bitcoin network (`mainnet` or `testnet`), several checks are performed:

-   The transaction is well formed.

-   The transaction only consumes unspent outputs with respect to the current (longest) blockchain, i.e., there is no block on the (longest) chain that consumes any of these outputs.

-   There is a positive transaction fee.

If at least one of these checks fails, the call is rejected.

If the transaction passes these tests, the transaction is forwarded to the specified Bitcoin network. Note that the function does not provide any guarantees that the transaction will make it into the mempool or that the transaction will ever appear in a block.

### `get_config`

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

