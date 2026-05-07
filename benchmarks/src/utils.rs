use bitcoin::block::Header;
use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};
use ic_btc_types::Block;
use std::str::FromStr;

/// Dummy address that receives outputs not directed at the target `address`.
const DUMMY_ADDRESS: &str = "bcrt1qxp8ercrmfxlu0s543najcj6fe6267j97tv7rgf";

/// Builds a chain of `num_blocks` blocks extending from the given header.
///
/// Each block has `num_transactions_per_block` transactions, each with
/// `num_outputs_per_transaction` outputs, except for the coinbase transaction.
///
/// When `num_transactions_per_block > 1`, the coinbase creates extra funding outputs
/// that the next block's non-coinbase transactions spend.
///
/// The first block in the chain only contains the coinbase transaction.
///
/// In each block except the first, there are `num_outputs_to_address_per_block` outputs going to
/// `address` which are created and permanently unspent, and `num_outputs_to_address_per_block` outputs
/// in the previous coinbase transaction which are spent:
/// - Each coinbase creates `num_outputs_to_address_per_block` funding outputs to `address`.
/// - There is at most one output that goes to the `address` in each non-coinbase transaction.
/// - All other outputs (remaining funding, other non-coinbase) go to a dummy address.
///
/// Because there is at most one output that goes to the `address` in each non-coinbase transaction,
/// `num_outputs_to_address_per_block` cannot be larger than `num_transactions_per_block - 1`.
pub fn build_chain_from(
    prev_header: Header,
    num_blocks: usize,
    num_transactions_per_block: usize,
    num_outputs_per_transaction: usize,
    num_outputs_to_address_per_block: usize,
    address: &bitcoin::Address,
    value_counter: &mut u64,
) -> Vec<Block> {
    assert!(
        num_transactions_per_block >= 1,
        "each block needs at least a coinbase transaction"
    );
    assert!(
        num_outputs_per_transaction >= 1,
        "each transaction needs at least one output"
    );
    let extra_txs = num_transactions_per_block - 1;
    assert!(
        num_outputs_to_address_per_block <= extra_txs,
        "num_outputs_to_address_per_block ({num_outputs_to_address_per_block}) exceeds the number of \
         non-coinbase transactions ({extra_txs})"
    );

    let dummy_address = bitcoin::Address::from_str(DUMMY_ADDRESS)
        .unwrap()
        .assume_checked();
    assert_ne!(
        address, &dummy_address,
        "address must differ from the dummy address ({DUMMY_ADDRESS})"
    );

    // Coinbase output layout (by vout index):
    //   [0, num_outputs_to_address_per_block)             → funding outputs (address)
    //   [num_outputs_to_address_per_block, extra_txs)     → funding outputs (dummy)

    let mut blocks = Vec::with_capacity(num_blocks);
    let mut prev = prev_header;
    let mut prev_coinbase_txid: Option<bitcoin::Txid> = None;

    for _ in 0..num_blocks {
        let mut coinbase = TransactionBuilder::coinbase();
        // Funding outputs: first `num_outputs_to_address_per_block` go to `address`,
        // the rest go to the dummy address.
        for i in 0..extra_txs {
            let dest = if i < num_outputs_to_address_per_block {
                address
            } else {
                &dummy_address
            };
            coinbase = coinbase.with_output(dest, *value_counter);
            *value_counter += 1;
        }
        let coinbase_tx = coinbase.build();
        let coinbase_txid = coinbase_tx.compute_txid();

        let mut builder = BlockBuilder::with_prev_header(prev).with_transaction(coinbase_tx);

        // Spend the previous block's funding outputs.
        if let Some(prev_txid) = prev_coinbase_txid {
            for i in 0..extra_txs {
                let mut tx = TransactionBuilder::new().with_input(
                    bitcoin::OutPoint {
                        txid: prev_txid,
                        vout: i as u32,
                    },
                    None,
                );
                // The first `num_outputs_to_address_per_block` txs spend from `address`
                // and create one output to `address` (permanently unspent).
                if i < num_outputs_to_address_per_block {
                    tx = tx.with_output(address, *value_counter);
                    *value_counter += 1;
                    for _ in 1..num_outputs_per_transaction {
                        tx = tx.with_output(&dummy_address, *value_counter);
                        *value_counter += 1;
                    }
                } else {
                    for _ in 0..num_outputs_per_transaction {
                        tx = tx.with_output(&dummy_address, *value_counter);
                        *value_counter += 1;
                    }
                }
                builder = builder.with_transaction(tx.build());
            }
        }

        let block = Block::new(builder.build());
        prev = *block.header();
        blocks.push(block);
        prev_coinbase_txid = Some(coinbase_txid);
    }
    blocks
}
