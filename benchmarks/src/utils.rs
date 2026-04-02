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
/// When `all_outputs_to_address` is `true`, every output goes to `address`.
/// When `false`, the coinbase first `num_outputs_per_transaction` outputs go to `address`;
/// the coinbase funding outputs go to a dummy address, except for the first one, which goes to
/// `address`, exercising UTXO removal for the target `address`.

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

    let other_address = if all_outputs_to_address {
        address.clone()
    } else {
        let addr = bitcoin::Address::from_str(DUMMY_ADDRESS)
            .unwrap()
            .assume_checked();
        assert_ne!(
            address, &addr,
            "address must differ from the dummy address ({DUMMY_ADDRESS})"
        );
        addr
    };
    let extra_txs = num_transactions_per_block - 1;

    let mut blocks = Vec::with_capacity(num_blocks);
    let mut prev = prev_header;
    let mut prev_coinbase_txid: Option<bitcoin::Txid> = None;

    for _ in 0..num_blocks {
        let mut coinbase = TransactionBuilder::coinbase();
        for _ in 0..num_outputs_per_transaction {
            coinbase = coinbase.with_output(address, *value_counter);
            *value_counter += 1;
        }
        // Extra funding outputs for the next block's non-coinbase transactions.
        // The first funding output goes to `address` so that spending it
        // exercises UTXO removal for the target address.
        for i in 0..extra_txs {
            let funding_address = if i == 0 { address } else { &other_address };
            coinbase = coinbase.with_output(funding_address, *value_counter);
            *value_counter += 1;
        }
        let coinbase_tx = coinbase.build();
        let coinbase_txid = coinbase_tx.compute_txid();

        let mut builder = BlockBuilder::with_prev_header(prev).with_transaction(coinbase_tx);

        // Spend the previous block's extra coinbase funding outputs.
        if let Some(prev_txid) = prev_coinbase_txid {
            for i in 0..extra_txs {
                let mut tx = TransactionBuilder::new().with_input(
                    bitcoin::OutPoint {
                        txid: prev_txid,
                        vout: (num_outputs_per_transaction + i) as u32,
                    },
                    None,
                );
                for _ in 0..num_outputs_per_transaction {
                    tx = tx.with_output(&other_address, *value_counter);
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
