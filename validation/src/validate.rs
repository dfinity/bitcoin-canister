use crate::constants::{
    no_pow_retargeting, pow_limit_bits, DIFFICULTY_ADJUSTMENT_INTERVAL, TEN_MINUTES,
};
use crate::{max_target, BlockHeight, HeaderStore, ValidateBlockError, ValidateHeaderError};
use bitcoin::block::Header;
use bitcoin::{CompactTarget, Network, Target, Transaction};
use std::collections::BTreeSet;
use std::time::Duration;

const ONE_HOUR: Duration = Duration::from_secs(3_600);

pub fn validate_block(
    network: &Network,
    store: &impl HeaderStore,
    block: &bitcoin::Block,
    current_time: Duration,
) -> Result<(), ValidateBlockError> {
    #[cfg(feature = "canbench-rs")]
    let _p = canbench_rs::bench_scope("validate");

    validate_header(network, store, &block.header, current_time)
        .map_err(ValidateBlockError::InvalidBlockHeader)
        .and_then(|()| validate_block_internal(block))
}

fn validate_block_internal(block: &bitcoin::Block) -> Result<(), ValidateBlockError> {
    #[cfg(feature = "canbench-rs")]
    let _p = canbench_rs::bench_scope("validate_block");

    // Check block like in
    // [bitcoin](https://github.com/rust-bitcoin/rust-bitcoin/blob/674ac57bce47e343d8f7c82e451aed5568766ba0/bitcoin/src/blockdata/block.rs#L126)
    let transactions = &block.txdata;
    if transactions.is_empty() {
        return Err(ValidateBlockError::NoTransactions);
    }

    if !transactions[0].is_coinbase() {
        return Err(ValidateBlockError::InvalidCoinbase);
    }

    {
        #[cfg(feature = "canbench-rs")]
        let _p = canbench_rs::bench_scope("validate_block/check_merkle_root");

        if !block.check_merkle_root() {
            return Err(ValidateBlockError::InvalidMerkleRoot);
        }
    }

    // TODO XC-497: evaluate performance impact of checking the witness commitment
    // like in [here](https://github.com/rust-bitcoin/rust-bitcoin/blob/674ac57bce47e343d8f7c82e451aed5568766ba0/bitcoin/src/blockdata/block.rs#L141)

    // Depart from the Rust bitcoin implementation because it's currently subject to
    // [CVE-2012-2459](https://bitcointalk.org/index.php?topic=102395)
    ensure_unique_transactions(&block.txdata)?;

    Ok(())
}

fn ensure_unique_transactions(transactions: &[Transaction]) -> Result<(), ValidateBlockError> {
    #[cfg(feature = "canbench-rs")]
    let _p = canbench_rs::bench_scope("validate_block/ensure_unique_transactions");

    let mut unique_normalized_txids: BTreeSet<_> = BTreeSet::new();
    for tx in transactions {
        let unique = unique_normalized_txids.insert(tx.compute_ntxid());
        if !unique {
            return Err(ValidateBlockError::DuplicateTransactions);
        }
    }
    Ok(())
}
pub fn validate_header(
    network: &Network,
    store: &impl HeaderStore,
    header: &Header,
    current_time: Duration,
) -> Result<(), ValidateHeaderError> {
    #[cfg(feature = "canbench-rs")]
    let _p = canbench_rs::bench_scope("validate_header");

    let prev_height = store.height();
    let prev_header = match store.get_with_block_hash(&header.prev_blockhash) {
        Some(result) => result,
        None => {
            return Err(ValidateHeaderError::PrevHeaderNotFound);
        }
    };

    is_timestamp_valid(store, header, current_time)?;

    let header_target = header.target();
    if header_target > max_target(&network) {
        return Err(ValidateHeaderError::TargetDifficultyAboveMax);
    }

    if header.validate_pow(header_target).is_err() {
        return Err(ValidateHeaderError::InvalidPoWForHeaderTarget);
    }

    let target = get_next_target(store, network, &prev_header, prev_height, header.time);
    if let Err(err) = header.validate_pow(target) {
        match err {
            bitcoin::block::ValidationError::BadProofOfWork => println!("bad proof of work"),
            bitcoin::block::ValidationError::BadTarget => println!("bad target"),
            _ => {}
        };
        return Err(ValidateHeaderError::InvalidPoWForComputedTarget);
    }
    Ok(())
}

/// Validates if a header's timestamp is valid.
/// Bitcoin Protocol Rules wiki https://en.bitcoin.it/wiki/Protocol_rules says,
/// "Reject if timestamp is the median time of the last 11 blocks or before"
/// "Block timestamp must not be more than two hours in the future"
fn is_timestamp_valid(
    store: &impl HeaderStore,
    header: &Header,
    current_time: Duration,
) -> Result<(), ValidateHeaderError> {
    timestamp_is_less_than_2h_in_future(Duration::from_secs(header.time as u64), current_time)?;
    let mut times = vec![];
    let mut current_header: Header = *header;
    let initial_hash = store.get_initial_hash();
    for _ in 0..11 {
        if let Some(prev_header) = store.get_with_block_hash(&current_header.prev_blockhash) {
            times.push(prev_header.time);
            if current_header.prev_blockhash == initial_hash {
                break;
            }
            current_header = prev_header;
        }
    }

    times.sort_unstable();
    let median = times[times.len() / 2];
    if header.time <= median {
        return Err(ValidateHeaderError::HeaderIsOld);
    }

    Ok(())
}

fn get_next_target(
    store: &impl HeaderStore,
    network: &Network,
    prev_header: &Header,
    prev_height: BlockHeight,
    timestamp: u32,
) -> Target {
    match network {
        Network::Testnet | Network::Testnet4 | Network::Regtest => {
            if (prev_height + 1) % DIFFICULTY_ADJUSTMENT_INTERVAL != 0 {
                // This if statements is reached only for Regtest and Testnet networks
                // Here is the quote from "https://en.bitcoin.it/wiki/Testnet"
                // "If no block has been found in 20 minutes, the difficulty automatically
                // resets back to the minimum for a single block, after which it
                // returns to its previous value."
                if timestamp > prev_header.time + TEN_MINUTES * 2 {
                    // If no block has been found in 20 minutes, then use the maximum difficulty
                    // target
                    max_target(&network)
                } else {
                    // If the block has been found within 20 minutes, then use the previous
                    // difficulty target that is not equal to the maximum difficulty target
                    Target::from_compact(find_next_difficulty_in_chain(
                        network,
                        store,
                        prev_header,
                        prev_height,
                    ))
                }
            } else {
                Target::from_compact(compute_next_difficulty(
                    network,
                    store,
                    prev_header,
                    prev_height,
                ))
            }
        }
        Network::Bitcoin | Network::Signet => Target::from_compact(compute_next_difficulty(
            network,
            store,
            prev_header,
            prev_height,
        )),
    }
}

/// This method is only valid when used for testnet and regtest networks.
/// As per "https://en.bitcoin.it/wiki/Testnet",
/// "If no block has been found in 20 minutes, the difficulty automatically
/// resets back to the minimum for a single block, after which it
/// returns to its previous value." This function is used to compute the
/// difficulty target in case the block has been found within 20
/// minutes.
fn find_next_difficulty_in_chain(
    network: &Network,
    store: &impl HeaderStore,
    prev_header: &Header,
    prev_height: BlockHeight,
) -> CompactTarget {
    // This is the maximum difficulty target for the network
    let pow_limit_bits = pow_limit_bits(&network);
    match network {
        Network::Testnet | Network::Testnet4 | Network::Regtest => {
            let mut current_header = *prev_header;
            let mut current_height = prev_height;
            let mut current_hash = current_header.block_hash();
            let initial_header_hash = store.get_initial_hash();

            // Keep traversing the blockchain backwards from the recent block to initial
            // header hash.
            loop {
                // Check if non-limit PoW found or it's time to adjust difficulty.
                if current_header.bits != pow_limit_bits
                    || current_height % DIFFICULTY_ADJUSTMENT_INTERVAL == 0
                {
                    return current_header.bits;
                }

                // Stop if we reach the initial header.
                if current_hash == initial_header_hash {
                    break;
                }

                // Traverse to the previous header.
                let prev_blockhash = current_header.prev_blockhash;
                current_header = store
                    .get_with_block_hash(&prev_blockhash)
                    .expect("previous header should be in the header store");
                // Update the current height and hash.
                current_height -= 1;
                current_hash = prev_blockhash;
            }
            pow_limit_bits
        }
        Network::Bitcoin | Network::Signet => pow_limit_bits,
    }
}

/// This function returns the difficulty target to be used for the current
/// header given the previous header
fn compute_next_difficulty(
    network: &Network,
    store: &impl HeaderStore,
    prev_header: &Header,
    prev_height: BlockHeight,
) -> CompactTarget {
    // Difficulty is adjusted only once in every interval of 2 weeks (2016 blocks)
    // If an interval boundary is not reached, then previous difficulty target is
    // returned Regtest network doesn't adjust PoW difficulty levels. For
    // regtest, simply return the previous difficulty target.

    let height = prev_height + 1;
    if height % DIFFICULTY_ADJUSTMENT_INTERVAL != 0 || no_pow_retargeting(&network) {
        return prev_header.bits;
    }
    // Computing the `last_adjustment_header`.
    // `last_adjustment_header` is the last header with height multiple of 2016
    let last_adjustment_height = if height < DIFFICULTY_ADJUSTMENT_INTERVAL {
        0
    } else {
        height - DIFFICULTY_ADJUSTMENT_INTERVAL
    };
    let last_adjustment_header = store
        .get_with_height(last_adjustment_height)
        .expect("Last adjustment header must exist");

    // Block Storm Fix
    // The mitigation consists of no longer applying the adjustment factor
    // to the last block of the previous difficulty period. Instead,
    // the first block of the difficulty period is used as the base.
    // See https://github.com/bitcoin/bips/blob/master/bip-0094.mediawiki#block-storm-fix
    let last = match network {
        Network::Testnet4 => last_adjustment_header.bits,
        _ => prev_header.bits,
    };

    // Computing the time interval between the last adjustment header time and
    // current time. The expected value timespan is 2 weeks assuming
    // the expected block time is 10 mins. But most of the time, the
    // timespan will deviate slightly from 2 weeks. Our goal is to
    // readjust the difficulty target so that the expected time taken for the next
    // 2016 blocks is again 2 weeks.
    // IMPORTANT: The bitcoin protocol allows for a roughly 3-hour window around
    // timestamp (1 hour in the past, 2 hours in the future) meaning that
    // the timespan can be negative on testnet networks.
    let last_adjustment_time = last_adjustment_header.time;
    let timespan = prev_header.time.saturating_sub(last_adjustment_time) as u64;

    CompactTarget::from_next_work_required(last, timespan, network)
}

fn timestamp_is_less_than_2h_in_future(
    block_time: Duration,
    current_time: Duration,
) -> Result<(), ValidateHeaderError> {
    let max_allowed_time = current_time + 2 * ONE_HOUR;

    if block_time > max_allowed_time {
        return Err(ValidateHeaderError::HeaderIsTooFarInFuture {
            block_time: block_time.as_secs(),
            max_allowed_time: max_allowed_time.as_secs(),
        });
    }

    Ok(())
}
