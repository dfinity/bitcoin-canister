use bitcoin::block::Header;
use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};
use ic_btc_types::Block;
use std::str::FromStr;

/// Dummy address that receives outputs not directed at the target `address`.
const DUMMY_ADDRESS: &str = "bcrt1qxp8ercrmfxlu0s543najcj6fe6267j97tv7rgf";

/// Builds a chain of `num_blocks` blocks extending from the given header.
///
/// Each block has `num_transactions_per_block` transactions, each with
/// `num_outputs_per_transaction` outputs.
///
/// When `all_outputs_to_address` is `true`, every output goes to `address`.
/// When `false`, only the coinbase transaction's outputs go to `address`;
/// non-coinbase transaction outputs go to a dummy address.
///
/// The first transaction in every block is a coinbase. When `num_transactions_per_block > 1`,
/// the coinbase creates extra funding outputs that the next block's non-coinbase transactions
/// spend. As a consequence, the very first block in the chain only contains the coinbase
/// transaction.
pub fn build_chain_from(
    prev_header: Header,
    num_blocks: usize,
    num_transactions_per_block: usize,
    num_outputs_per_transaction: usize,
    all_outputs_to_address: bool,
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

    let dummy_address = bitcoin::Address::from_str(DUMMY_ADDRESS)
        .unwrap()
        .assume_checked();
    assert_ne!(
        address, &dummy_address,
        "address must differ from the dummy address ({DUMMY_ADDRESS})"
    );
    let extra_txs = num_transactions_per_block - 1;

    let mut blocks = Vec::with_capacity(num_blocks);
    let mut prev = prev_header;
    let mut prev_coinbase_txid: Option<bitcoin::Txid> = None;

    for _ in 0..num_blocks {
        // Coinbase outputs always go to `address`.
        let mut coinbase = TransactionBuilder::coinbase();
        for _ in 0..num_outputs_per_transaction {
            coinbase = coinbase.with_output(address, *value_counter);
            *value_counter += 1;
        }
        // Extra funding outputs for the next block's non-coinbase transactions.
        for _ in 0..extra_txs {
            coinbase = coinbase.with_output(&dummy_address, *value_counter);
            *value_counter += 1;
        }
        let coinbase_tx = coinbase.build();
        let coinbase_txid = coinbase_tx.compute_txid();

        let mut builder = BlockBuilder::with_prev_header(prev).with_transaction(coinbase_tx);

        // Spend the previous block's extra coinbase funding outputs.
        if let Some(prev_txid) = prev_coinbase_txid {
            let output_address = if all_outputs_to_address {
                address
            } else {
                &dummy_address
            };
            for i in 0..extra_txs {
                let mut tx = TransactionBuilder::new().with_input(
                    bitcoin::OutPoint {
                        txid: prev_txid,
                        vout: (num_outputs_per_transaction + i) as u32,
                    },
                    None,
                );
                // Spend one coinbase output to `address` to exercise UTXO removal.
                if i == 0 {
                    tx = tx.with_input(
                        bitcoin::OutPoint {
                            txid: prev_txid,
                            vout: 0,
                        },
                        None,
                    );
                }
                for _ in 0..num_outputs_per_transaction {
                    tx = tx.with_output(output_address, *value_counter);
                    *value_counter += 1;
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
