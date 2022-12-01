use crate::{
    charge_cycles,
    runtime::{performance_counter, print},
    state::{FeePercentilesCache, State},
    types::{Block, Transaction},
    unstable_blocks::{self, UnstableBlocks},
    verify_has_enough_cycles, with_state, with_state_mut,
};
use ic_btc_types::MillisatoshiPerByte;

/// The number of transactions to include in the percentiles calculation.
const NUM_TRANSACTIONS: u32 = 10_000;

/// Returns the 100 fee percentiles of the chain's 10,000 most recent transactions.
pub fn get_current_fee_percentiles() -> Vec<MillisatoshiPerByte> {
    verify_has_enough_cycles(with_state(|s| s.fees.get_current_fee_percentiles_maximum));
    charge_cycles(with_state(|s| s.fees.get_current_fee_percentiles));

    let res = with_state_mut(|s| get_current_fee_percentiles_internal(s, NUM_TRANSACTIONS));

    // Observe instruction count.
    let ins_total = performance_counter();
    with_state_mut(|s| {
        s.metrics
            .get_current_fee_percentiles_total
            .observe(ins_total)
    });
    print(&format!(
        "[INSTRUCTION COUNT] get_current_fee_percentiles: {}",
        ins_total
    ));
    res
}

fn get_current_fee_percentiles_internal(
    state: &mut State,
    number_of_transactions: u32,
) -> Vec<MillisatoshiPerByte> {
    let main_chain = unstable_blocks::get_main_chain(&state.unstable_blocks);
    let tip_block_hash = main_chain.tip().block_hash();

    // If fee percentiles were already cached, then return the cached results.
    if let Some(cache) = &state.fee_percentiles_cache {
        if cache.tip_block_hash == tip_block_hash {
            return cache.fee_percentiles.clone();
        }
    }

    // If tip block changed recalculate and cache results.
    let fee_percentiles = percentiles(get_fees_per_byte(
        main_chain.into_chain(),
        &state.unstable_blocks,
        number_of_transactions,
    ));

    state.fee_percentiles_cache = Some(FeePercentilesCache {
        tip_block_hash,
        fee_percentiles: fee_percentiles.clone(),
    });

    fee_percentiles
}

/// Computes the fees per byte of the last `number_of_transactions` transactions on the main chain.
/// Fees are returned in a reversed order, starting with the most recent ones, followed by the older ones.
/// Eg. for transactions [..., Tn-2, Tn-1, Tn] fees would be [Fn, Fn-1, Fn-2, ...].
fn get_fees_per_byte(
    main_chain: Vec<&Block>,
    unstable_blocks: &UnstableBlocks,
    number_of_transactions: u32,
) -> Vec<MillisatoshiPerByte> {
    let mut fees = Vec::new();
    let mut tx_i = 0;
    for block in main_chain.iter().rev() {
        if tx_i >= number_of_transactions {
            break;
        }
        for tx in block.txdata() {
            if tx_i >= number_of_transactions {
                break;
            }
            tx_i += 1;
            if let Some(fee) = get_tx_fee_per_byte(tx, unstable_blocks) {
                fees.push(fee);
            }
        }
    }
    fees
}

/// Computes the fees per byte of the given transaction.
fn get_tx_fee_per_byte(
    tx: &Transaction,
    unstable_blocks: &UnstableBlocks,
) -> Option<MillisatoshiPerByte> {
    if tx.is_coin_base() {
        // Coinbase transactions do not have a fee.
        return None;
    }

    let mut satoshi = 0;
    for tx_in in tx.input() {
        let outpoint = (&tx_in.previous_output).into();
        satoshi += unstable_blocks
            .get_tx_out(&outpoint)
            .unwrap_or_else(|| panic!("tx out of outpoint {:?} must exist", outpoint))
            .0
            .value;
    }
    for tx_out in tx.output() {
        satoshi -= tx_out.value;
    }

    if tx.size() > 0 {
        // Don't use floating point division to avoid non-determinism.
        Some(((1000 * satoshi) / tx.size() as u64) as MillisatoshiPerByte)
    } else {
        // Calculating fee is not possible for a zero-size invalid transaction.
        None
    }
}

/// Compute percentiles of input values.
///
/// Returns 101 bucket to cover the percentiles range `[0, 100]`.
/// Uses standard nearest-rank estimation method, inclusive, with the extension of a 0th percentile.
/// See https://en.wikipedia.org/wiki/Percentile#The_nearest-rank_method.
fn percentiles(mut values: Vec<u64>) -> Vec<u64> {
    if values.is_empty() {
        return vec![];
    }
    values.sort_unstable();
    const MAX_PERCENTILE: u32 = 100;
    let ceil_div = |a, b| a / b + if a % b == 0 { 0 } else { 1 };
    (0..MAX_PERCENTILE + 1)
        .map(|p| {
            // `ordinal_rank = ceil(p/100 * n)`.
            let ordinal_rank = ceil_div(p * values.len() as u32, MAX_PERCENTILE);
            let index = std::cmp::max(0, ordinal_rank as i32 - 1);
            values[index as usize]
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        genesis_block, state,
        test_utils::{random_p2pkh_address, BlockBuilder, TransactionBuilder},
        types::{Config, Fees, Network, OutPoint},
        with_state,
    };
    use ic_btc_types::Satoshi;
    use std::iter::FromIterator;

    /// Covers an inclusive range of `[0, 100]` percentiles.
    const PERCENTILE_BUCKETS: usize = 101;

    #[test]
    fn percentiles_empty_input() {
        assert_eq!(percentiles(vec![]).len(), 0);
    }

    #[test]
    fn percentiles_nearest_rank_method_simple_example() {
        let percentiles = percentiles(vec![15, 20, 35, 40, 50]);
        assert_eq!(percentiles.len(), PERCENTILE_BUCKETS);
        assert_eq!(percentiles[0..21], [15; 21]);
        assert_eq!(percentiles[21..41], [20; 20]);
        assert_eq!(percentiles[41..61], [35; 20]);
        assert_eq!(percentiles[61..81], [40; 20]);
        assert_eq!(percentiles[81..101], [50; 20]);
    }

    #[test]
    fn percentiles_small_input() {
        let percentiles = percentiles(vec![5, 4, 3, 2, 1]);
        assert_eq!(percentiles.len(), PERCENTILE_BUCKETS);
        assert_eq!(percentiles[0..21], [1; 21]);
        assert_eq!(percentiles[21..41], [2; 20]);
        assert_eq!(percentiles[41..61], [3; 20]);
        assert_eq!(percentiles[61..81], [4; 20]);
        assert_eq!(percentiles[81..101], [5; 20]);
    }

    #[test]
    fn percentiles_big_input() {
        let mut input = vec![];
        input.extend(vec![5; 1000]);
        input.extend(vec![4; 1000]);
        input.extend(vec![3; 1000]);
        input.extend(vec![2; 1000]);
        input.extend(vec![1; 1000]);
        let percentiles = percentiles(input);
        assert_eq!(percentiles.len(), PERCENTILE_BUCKETS);
        assert_eq!(percentiles[0..21], [1; 21]);
        assert_eq!(percentiles[21..41], [2; 20]);
        assert_eq!(percentiles[41..61], [3; 20]);
        assert_eq!(percentiles[61..81], [4; 20]);
        assert_eq!(percentiles[81..101], [5; 20]);
    }

    #[test]
    /// Given the input [1, 2, ..., 1000], the test ensures that the computed fees
    /// are [10, 20, ..., 1000].
    fn percentiles_sequential_numbers() {
        let input = Vec::from_iter(1..1_001);
        let percentiles = percentiles(input);
        assert_eq!(percentiles.len(), PERCENTILE_BUCKETS);
        assert_eq!(percentiles[0], 1);
        assert_eq!(percentiles[1], 10);
        assert_eq!(percentiles[25], 250);
        assert_eq!(percentiles[50], 500);
        assert_eq!(percentiles[75], 750);
        assert_eq!(percentiles[100], 1_000);
        let mut expected = vec![1];
        expected.extend_from_slice(&Vec::from_iter((10..1_001).step_by(10)));
        assert_eq!(percentiles, expected);
    }

    // Generates a chain of blocks:
    // - genesis block receives a coinbase transaction on address_1 with initial_balance
    // - follow-up blocks transfer payments from address_1 to address_2 with a specified fee
    // Fee is choosen to be a multiple of transaction size to have round values of fee.
    fn generate_blocks(initial_balance: Satoshi, number_of_blocks: u32) -> Vec<Block> {
        let network = Network::Regtest;
        let mut blocks = Vec::new();

        let pay: Satoshi = 1;
        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);

        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, initial_balance)
            .build();
        let block_0 = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(coinbase_tx.clone())
            .build();
        blocks.push(block_0.clone());

        let mut balance = initial_balance;
        let mut previous_tx = coinbase_tx;
        let mut previous_block = block_0;

        for i in 0..number_of_blocks {
            // For testing purposes every transaction has 1 Satoshi higher fee than the previous one, starting with 0 satoshi.
            // Each fake transaction is 119 bytes in size.
            // I.e. a sequence of fees [0, 1, 2, 3] satoshi converts to [0, 8, 16, 25] milisatoshi per byte.
            // To estimate initial balance:
            // number_of_blocks * (number_of_blocks + 1) / 2
            let fee = i as Satoshi;
            let change = match balance.checked_sub(pay + fee) {
                Some(value) => value,
                None => panic!(
                    "There is not enough balance of {} Satoshi to perform transaction #{} with fee of {} satoshi",
                    balance, i, fee
                ),
            };

            let tx = TransactionBuilder::new()
                .with_input(OutPoint::new(previous_tx.txid(), 0))
                .with_output(&address_1, change)
                .with_output(&address_2, pay)
                .build();
            let block = BlockBuilder::with_prev_header(previous_block.header())
                .with_transaction(tx.clone())
                .build();
            blocks.push(block.clone());

            balance = change;
            previous_tx = tx;
            previous_block = block;
        }

        blocks
    }

    fn init_state(blocks: Vec<Block>, stability_threshold: u128) {
        crate::init(Config {
            stability_threshold,
            network: Network::Regtest,
            ..Default::default()
        });

        with_state_mut(|state| {
            for (i, block) in blocks.into_iter().enumerate() {
                state::insert_block(state, block).unwrap();
                if i % 1000 == 0 {
                    println!("processed block: {}", i);
                }
            }

            state::ingest_stable_blocks_into_utxoset(state);
        });
    }

    #[test]
    fn get_current_fee_percentiles_requested_number_of_txs_is_greater_than_number_of_actual_txs() {
        let number_of_blocks = 5;
        let blocks = generate_blocks(10_000, number_of_blocks);
        let number_of_transactions = 10_000;
        let stability_threshold = blocks.len() as u128;
        init_state(blocks, stability_threshold);
        with_state(|state| {
            let main_chain = unstable_blocks::get_main_chain(&state.unstable_blocks).into_chain();

            let fees = get_fees_per_byte(
                main_chain.clone(),
                &state.unstable_blocks,
                number_of_transactions as u32,
            );

            // Initial transactions' fees [0, 1, 2, 3, 4] satoshi, with 119 bytes of transaction size
            // transfer into [0, 8, 16, 25, 33] millisatoshi per byte fees in chronological order.
            assert_eq!(fees.len(), number_of_blocks as usize);
            // Fees are in a reversed order, in millisatoshi per byte units.
            assert_eq!(fees, vec![33, 25, 16, 8, 0]);
        });

        let percentiles = get_current_fee_percentiles();
        assert_eq!(percentiles.len(), PERCENTILE_BUCKETS);
        assert_eq!(percentiles[0..21], [0; 21]);
        assert_eq!(percentiles[21..41], [8; 20]);
        assert_eq!(percentiles[41..61], [16; 20]);
        assert_eq!(percentiles[61..81], [25; 20]);
        assert_eq!(percentiles[81..101], [33; 20]);
    }

    #[test]
    fn get_current_fee_percentiles_requested_number_of_txs_is_less_than_number_of_actual_txs() {
        let number_of_blocks = 8;
        let blocks = generate_blocks(10_000, number_of_blocks);
        let stability_threshold = blocks.len() as u128;
        init_state(blocks, stability_threshold);

        with_state_mut(|state| {
            let main_chain = unstable_blocks::get_main_chain(&state.unstable_blocks).into_chain();

            let number_of_transactions = 4;
            let fees = get_fees_per_byte(
                main_chain.clone(),
                &state.unstable_blocks,
                number_of_transactions,
            );
            // Initial transactions' fees [0, 1, 2, 3, 4, 5, 6, 7, 8] satoshi, with 119 bytes of transaction size
            // transfer into [0, 8, 16, 25, 33, 42, 50, 58] millisatoshi per byte fees in chronological order.
            // Extracted fees contain only last 4 transaction fees in a reversed order.
            assert_eq!(fees.len(), number_of_transactions as usize);
            // Fees are in a reversed order, in millisatoshi per byte units.
            assert_eq!(fees, vec![58, 50, 42, 33]);

            let percentiles = get_current_fee_percentiles_internal(state, 4);
            assert_eq!(percentiles.len(), PERCENTILE_BUCKETS);
            assert_eq!(percentiles[0..26], [33; 26]);
            assert_eq!(percentiles[26..51], [42; 25]);
            assert_eq!(percentiles[51..76], [50; 25]);
            assert_eq!(percentiles[76..101], [58; 25]);
        });
    }

    #[test]
    fn get_current_fee_percentiles_requested_number_of_txs_is_equal_to_the_number_of_actual_txs() {
        let number_of_blocks = 5;
        let blocks = generate_blocks(10_000, number_of_blocks);
        let stability_threshold = blocks.len() as u128;
        init_state(blocks, stability_threshold);

        with_state_mut(|state| {
            let main_chain = unstable_blocks::get_main_chain(&state.unstable_blocks).into_chain();

            let number_of_transactions = 5;
            let fees = get_fees_per_byte(
                main_chain.clone(),
                &state.unstable_blocks,
                number_of_transactions,
            );
            let percentiles = get_current_fee_percentiles_internal(state, number_of_transactions);

            // Initial transactions' fees [0, 1, 2, 3, 4] satoshi, with 119 bytes of transaction size
            // transfer into [0, 8, 16, 25, 33] millisatoshi per byte fees in chronological order.
            assert_eq!(fees.len(), number_of_blocks as usize);
            // Fees are in a reversed order, in millisatoshi per byte units.
            assert_eq!(fees, vec![33, 25, 16, 8, 0]);

            assert_eq!(percentiles.len(), PERCENTILE_BUCKETS);
            assert_eq!(percentiles[0..21], [0; 21]);
            assert_eq!(percentiles[21..41], [8; 20]);
            assert_eq!(percentiles[41..61], [16; 20]);
            assert_eq!(percentiles[61..81], [25; 20]);
            assert_eq!(percentiles[81..101], [33; 20]);
        });
    }

    #[test]
    fn get_current_fee_percentiles_big_input() {
        let number_of_blocks = 1_000;
        let initial_balance = 500_500; // number_of_blocks * (number_of_blocks + 1) / 2
        let blocks = generate_blocks(initial_balance, number_of_blocks);
        let stability_threshold = blocks.len() as u128;
        init_state(blocks, stability_threshold);

        with_state_mut(|state| {
            let main_chain = unstable_blocks::get_main_chain(&state.unstable_blocks).into_chain();

            let number_of_transactions = 5;
            let fees = get_fees_per_byte(
                main_chain.clone(),
                &state.unstable_blocks,
                number_of_transactions,
            );
            let percentiles = get_current_fee_percentiles_internal(state, number_of_transactions);

            // Initial transactions' fees [0, 1, 2, 3, ...] satoshi, with 119 bytes of transaction size
            // transfer into [0, 8, 16, 25, ...] millisatoshi per byte fees in chronological order.
            assert_eq!(fees.len(), number_of_transactions as usize);
            // Fees are in a reversed order, in millisatoshi per byte units.
            // Eg. the fee of 999 satoshi for transaction of 119 bytes converts to
            // 1000 * 999 / 119 = 8394 millisatosi/bite.
            assert_eq!(fees, vec![8394, 8386, 8378, 8369, 8361]);

            assert_eq!(percentiles.len(), PERCENTILE_BUCKETS);
            assert_eq!(percentiles[0..21], [8361; 21]);
            assert_eq!(percentiles[21..41], [8369; 20]);
            assert_eq!(percentiles[41..61], [8378; 20]);
            assert_eq!(percentiles[61..81], [8386; 20]);
            assert_eq!(percentiles[81..101], [8394; 20]);
        });
    }

    #[test]
    fn get_current_fee_percentiles_no_transactions() {
        let number_of_blocks = 0;
        let blocks = generate_blocks(10_000, number_of_blocks);
        let stability_threshold = blocks.len() as u128;
        init_state(blocks, stability_threshold);

        with_state_mut(|state| {
            let main_chain = unstable_blocks::get_main_chain(&state.unstable_blocks).into_chain();

            let number_of_transactions = 10_000;
            let fees = get_fees_per_byte(
                main_chain.clone(),
                &state.unstable_blocks,
                number_of_transactions,
            );
            assert_eq!(fees.len(), 0);
        });

        let percentiles = get_current_fee_percentiles();
        assert_eq!(percentiles.len(), 0);
    }

    #[test]
    fn get_current_fee_percentiles_from_utxos() {
        let number_of_blocks = 5;
        let number_of_transactions = 10_000;
        let blocks = generate_blocks(10_000, number_of_blocks);
        let stability_threshold = 1;
        init_state(blocks, stability_threshold);

        with_state_mut(|state| {
            let main_chain = unstable_blocks::get_main_chain(&state.unstable_blocks).into_chain();
            let fees = get_fees_per_byte(
                main_chain.clone(),
                &state.unstable_blocks,
                number_of_transactions,
            );

            // Initial transactions' fees [0, 1, 2, 3, 4] satoshi, with 119 bytes of transaction size
            // transfer into [0, 8, 16, 25, 33] millisatoshi per byte fees in chronological order.
            // But only 2 last transactions are placed in unstable blocks that form a main chain.
            // All the rest of the blocks are partially stored in UTXO set, which does not have information
            // about the sequence and input values, which does not allow to compute the fee.
            assert_eq!(fees.len(), 2);
            // Fees are in a reversed order, in millisatoshi per byte units.
            assert_eq!(fees, vec![33, 25]);
        });

        let percentiles = get_current_fee_percentiles();
        assert_eq!(percentiles.len(), PERCENTILE_BUCKETS);
        assert_eq!(percentiles[0..51], [25; 51]);
        assert_eq!(percentiles[51..101], [33; 50]);
    }

    #[test]
    fn get_current_fee_percentiles_caches_results() {
        let number_of_blocks = 5;
        let blocks = generate_blocks(10_000, number_of_blocks);
        let stability_threshold = 1;
        init_state(blocks, stability_threshold);

        let percentiles = get_current_fee_percentiles();
        assert_eq!(percentiles.len(), PERCENTILE_BUCKETS);
        assert_eq!(percentiles[0..51], [25; 51]);
        assert_eq!(percentiles[51..101], [33; 50]);

        // Percentiles are cached.
        with_state(|state| {
            assert_eq!(
                state.fee_percentiles_cache.clone().unwrap().fee_percentiles,
                percentiles
            );
        });
    }

    #[test]
    fn charges_cycles() {
        crate::init(Config {
            fees: Fees {
                get_current_fee_percentiles: 10,
                ..Default::default()
            },
            ..Default::default()
        });

        get_current_fee_percentiles();

        assert_eq!(crate::runtime::get_cycles_balance(), 10);
    }
}
