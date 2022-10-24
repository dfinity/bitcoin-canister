use crate::{
    types::{Address, Block, OutPoint, Utxo},
    unstable_blocks::UnstableBlocks,
    UtxoSet,
};
use std::collections::BTreeSet;
use std::iter::Peekable;

/// A struct that tracks the UTXO set of a given address.
///
/// Given a reference to a full UTXO set, it is able to simulate adding
/// additional transactions and its impact on the UTXO set of `address`, which
/// is used for computing the UTXOs of an address at varying heights.
pub struct AddressUtxoSet<'a> {
    // The address to track the UTXOs of.
    address: Address,

    // A reference to the (full) underlying UTXO set.
    full_utxo_set: &'a UtxoSet,

    unstable_blocks: &'a UnstableBlocks,

    // Added UTXOs that are not present in the underlying UTXO set.
    added_utxos: BTreeSet<Utxo>,

    // Outpoints of the removed UTXOs that are still present in the underlying UTXO set.
    removed_outpoints: BTreeSet<OutPoint>,
}

impl<'a> AddressUtxoSet<'a> {
    /// Initialize an `AddressUtxoSet` that tracks the UTXO set of `address`.
    pub fn new(
        address: Address,
        full_utxo_set: &'a UtxoSet,
        unstable_blocks: &'a UnstableBlocks,
    ) -> Self {
        Self {
            address,
            full_utxo_set,
            unstable_blocks,
            removed_outpoints: BTreeSet::new(),
            added_utxos: BTreeSet::new(),
        }
    }

    pub fn apply_block(&mut self, block: &Block) {
        for outpoint in self
            .unstable_blocks
            .get_removed_outpoints(&block.block_hash().to_vec(), &self.address)
        {
            self.removed_outpoints.insert(outpoint.clone());
        }

        for outpoint in self
            .unstable_blocks
            .get_added_outpoints(&block.block_hash().to_vec(), &self.address)
        {
            let (txout, height) = self
                .unstable_blocks
                .get_tx_out(outpoint)
                .unwrap_or_else(|| {
                    panic!(
                        "tx out for outpoint {:?} must exist in added outpoints",
                        outpoint
                    );
                });
            self.added_utxos.insert(Utxo {
                outpoint: outpoint.clone(),
                value: txout.value,
                height,
            });
        }
    }

    pub fn into_iter(self, offset: Option<Utxo>) -> impl Iterator<Item = Utxo> + 'a {
        let stable_utxos = self.full_utxo_set.get_address_utxos(&self.address, &offset);

        let unstable_utxos = self
            .added_utxos
            .into_iter()
            .filter(move |utxo| match &offset {
                Some(offset) => utxo >= offset,
                None => true,
            });

        let iter = MultiIter::new(stable_utxos, unstable_utxos);

        let removed_outpoints = self.removed_outpoints;
        iter.into_iter().filter_map(move |utxo| {
            if removed_outpoints.contains(&utxo.outpoint) {
                return None;
            }

            Some(utxo)
        })
    }
}

/// An iterator that consumes multiple iterators and returns their items interleaved in sorted order.
/// The iterators themselves must be sorted.
pub struct MultiIter<T, A: Iterator<Item = T>, B: Iterator<Item = T>> {
    a: Peekable<A>,
    b: Peekable<B>,
}

impl<T, A: Iterator<Item = T>, B: Iterator<Item = T>> MultiIter<T, A, B> {
    fn new(a: A, b: B) -> Self {
        Self {
            a: a.peekable(),
            b: b.peekable(),
        }
    }
}

impl<T: PartialOrd, A: Iterator<Item = T>, B: Iterator<Item = T>> Iterator for MultiIter<T, A, B> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let next_a = self.a.peek();
        let next_b = self.b.peek();

        match (next_a, next_b) {
            (Some(next_a), Some(next_b)) => {
                if next_a < next_b {
                    self.a.next()
                } else {
                    self.b.next()
                }
            }
            (Some(_), None) => self.a.next(),
            (None, Some(_)) => self.b.next(),
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{random_p2pkh_address, BlockBuilder, TransactionBuilder};
    use crate::{
        types::{Network, OutPoint},
        unstable_blocks,
    };

    #[test]
    fn add_tx_to_empty_utxo() {
        // Create some BTC addresses.
        let address_1 = random_p2pkh_address(Network::Mainnet);

        let utxo_set = UtxoSet::new(Network::Mainnet);

        // Create a genesis block where 1000 satoshis are given to address 1.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();

        let block_0 = BlockBuilder::genesis()
            .with_transaction(coinbase_tx.clone())
            .build();

        let unstable_blocks = UnstableBlocks::new(&utxo_set, 2, block_0.clone());

        let mut address_utxo_set = AddressUtxoSet::new(address_1, &utxo_set, &unstable_blocks);

        address_utxo_set.apply_block(&block_0);

        // Address should have that data.
        assert_eq!(
            address_utxo_set.into_iter(None).collect::<Vec<_>>(),
            vec![Utxo {
                outpoint: OutPoint {
                    txid: coinbase_tx.txid(),
                    vout: 0
                },
                value: 1000,
                height: 0
            }]
        );
    }

    #[test]
    fn add_tx_then_transfer() {
        // Create some BTC addresses.
        let address_1 = random_p2pkh_address(Network::Mainnet);
        let address_2 = random_p2pkh_address(Network::Mainnet);

        let utxo_set = UtxoSet::new(Network::Mainnet);

        // Create a genesis block where 1000 satoshis are given to address 1.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        let block_0 = BlockBuilder::genesis()
            .with_transaction(coinbase_tx.clone())
            .build();

        // Extend block 0 with block 1 that spends the 1000 satoshis and gives them to address 2.
        let tx = TransactionBuilder::new()
            .with_input(OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_2, 1000)
            .build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .with_transaction(tx.clone())
            .build();

        let mut unstable_blocks = UnstableBlocks::new(&utxo_set, 2, block_0.clone());
        unstable_blocks::push(&mut unstable_blocks, &utxo_set, block_1.clone()).unwrap();

        let mut address_utxo_set = AddressUtxoSet::new(address_1, &utxo_set, &unstable_blocks);
        address_utxo_set.apply_block(&block_0);
        address_utxo_set.apply_block(&block_1);

        assert_eq!(address_utxo_set.into_iter(None).collect::<Vec<_>>(), vec![]);

        let mut address_2_utxo_set = AddressUtxoSet::new(address_2, &utxo_set, &unstable_blocks);
        address_2_utxo_set.apply_block(&block_0);
        address_2_utxo_set.apply_block(&block_1);

        assert_eq!(
            address_2_utxo_set.into_iter(None).collect::<Vec<_>>(),
            vec![Utxo {
                outpoint: OutPoint {
                    txid: tx.txid(),
                    vout: 0
                },
                value: 1000,
                height: 1
            }]
        );
    }

    #[test]
    fn spending_multiple_inputs() {
        let network = Network::Mainnet;

        // Create some BTC addresses.
        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);

        // Create a genesis block where 2000 satoshis are given to address 1
        // in two different outputs.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .with_output(&address_1, 1000)
            .build();
        let block_0 = BlockBuilder::genesis()
            .with_transaction(coinbase_tx.clone())
            .build();

        // Address 1 spends both outputs in a single transaction.
        let tx = TransactionBuilder::new()
            .with_input(OutPoint::new(coinbase_tx.txid(), 0))
            .with_input(OutPoint::new(coinbase_tx.txid(), 1))
            .with_output(&address_2, 1500)
            .with_output(&address_1, 400)
            .build();
        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .with_transaction(tx.clone())
            .build();

        // Process the blocks.
        let utxo_set = UtxoSet::new(Network::Mainnet);
        let mut unstable_blocks = UnstableBlocks::new(&utxo_set, 2, block_0.clone());
        unstable_blocks::push(&mut unstable_blocks, &utxo_set, block_1.clone()).unwrap();

        let mut address_1_utxo_set = AddressUtxoSet::new(address_1, &utxo_set, &unstable_blocks);
        address_1_utxo_set.apply_block(&block_0);
        address_1_utxo_set.apply_block(&block_1);

        let mut address_2_utxo_set = AddressUtxoSet::new(address_2, &utxo_set, &unstable_blocks);
        address_2_utxo_set.apply_block(&block_0);
        address_2_utxo_set.apply_block(&block_1);

        // Address 1 should have one UTXO corresponding to the remaining amount
        // it gave back to itself.
        assert_eq!(
            address_1_utxo_set.into_iter(None).collect::<Vec<_>>(),
            vec![Utxo {
                outpoint: OutPoint {
                    txid: tx.txid(),
                    vout: 1
                },
                value: 400,
                height: 1
            }]
        );

        // Address 2 should receive 1500 Satoshi
        assert_eq!(
            address_2_utxo_set.into_iter(None).collect::<Vec<_>>(),
            vec![Utxo {
                outpoint: OutPoint {
                    txid: tx.txid(),
                    vout: 0
                },
                value: 1500,
                height: 1
            }]
        );
    }
}
