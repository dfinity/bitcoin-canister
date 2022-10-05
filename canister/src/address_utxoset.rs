use crate::{
    state::UtxoSet,
    types::{Address, Block, OutPoint, Storable, TxOut},
    unstable_blocks::UnstableBlocks,
};
use bitcoin::Script;
use ic_btc_types::{Height, Utxo};
use std::collections::{BTreeMap, BTreeSet};

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
    added_utxos: BTreeMap<OutPoint, (TxOut, Height)>,

    // Removed UTXOs that are still present in the underlying UTXO set.
    removed_utxos: BTreeMap<OutPoint, (TxOut, Height)>,
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
            removed_utxos: BTreeMap::new(),
            added_utxos: BTreeMap::new(),
        }
    }

    pub fn apply_block(&mut self, block: &Block) {
        for outpoint in self
            .unstable_blocks
            .get_removed_outpoints(&block.block_hash().to_vec(), &self.address)
        {
            let (txout, height) = self.unstable_blocks.get_tx_out(outpoint).unwrap();
            self.removed_utxos
                .insert(outpoint.clone(), (txout.clone(), height));
        }

        for outpoint in self
            .unstable_blocks
            .get_added_outpoints(&block.block_hash().to_vec(), &self.address)
        {
            let (txout, height) = self.unstable_blocks.get_tx_out(outpoint).unwrap();
            self.added_utxos
                .insert(outpoint.clone(), (txout.clone(), height));
        }
    }

    pub fn into_vec(self, offset: Option<(Height, OutPoint)>) -> Vec<Utxo> {
        // Retrieve all the UTXOs of the address from the underlying UTXO set.
        let mut set: BTreeSet<_> = self
            .full_utxo_set
            .address_to_outpoints
            .range(
                self.address.to_bytes(),
                offset.as_ref().map(|x| x.to_bytes()),
            )
            .filter_map(|(k, _)| {
                let (_, _, outpoint) = <(Address, Height, OutPoint)>::from_bytes(k);

                // Skip this outpoint if it has been removed in an unstable block.
                if self.removed_utxos.contains_key(&outpoint) {
                    return None;
                }

                let (txout, height) = self
                    .full_utxo_set
                    .utxos
                    .get(&outpoint)
                    .expect("outpoint must exist");

                Some(((height, outpoint).to_bytes(), txout))
            })
            .collect();

        let added_utxos: BTreeMap<_, _> = self
            .added_utxos
            .clone()
            .into_iter()
            .filter(|(outpoint, _)| !self.removed_utxos.contains_key(outpoint))
            .collect();

        // Include all the newly added UTXOs for that address that are "after" the optional offset.
        //
        // First, the UTXOs are encoded in a way that's consistent with the stable UTXO set
        // to preserve the ordering.
        let mut added_utxos_encoded: BTreeMap<_, _> = added_utxos
            .into_iter()
            .map(|(outpoint, (txout, height))| ((height, outpoint).to_bytes(), txout))
            .collect();

        // If an offset is specified, then discard the UTXOs before the offset.
        let added_utxos_encoded = match offset {
            Some(offset) => added_utxos_encoded.split_off(&offset.to_bytes()),
            None => added_utxos_encoded,
        };

        for (height_and_outpoint, txout) in added_utxos_encoded {
            if let Ok(address) = Address::from_script(
                &Script::from(txout.script_pubkey.clone()),
                self.full_utxo_set.network,
            ) {
                if address == self.address {
                    assert!(
                        set.insert((height_and_outpoint, txout)),
                        "Cannot overwrite existing outpoint"
                    );
                }
            }
        }

        set.into_iter()
            .map(|(height_and_outpoint, txout)| {
                let (height, outpoint) = <(Height, OutPoint)>::from_bytes(height_and_outpoint);
                Utxo {
                    outpoint: ic_btc_types::OutPoint {
                        txid: outpoint.txid.to_vec(),
                        vout: outpoint.vout,
                    },
                    value: txout.value,
                    height,
                }
            })
            .collect()
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
    use ic_btc_types::OutPoint as PublicOutPoint;

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
            address_utxo_set.into_vec(None),
            vec![Utxo {
                outpoint: PublicOutPoint {
                    txid: coinbase_tx.txid().to_vec(),
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

        assert_eq!(address_utxo_set.into_vec(None), vec![]);

        let mut address_2_utxo_set = AddressUtxoSet::new(address_2, &utxo_set, &unstable_blocks);
        address_2_utxo_set.apply_block(&block_0);
        address_2_utxo_set.apply_block(&block_1);

        assert_eq!(
            address_2_utxo_set.into_vec(None),
            vec![Utxo {
                outpoint: PublicOutPoint {
                    txid: tx.txid().to_vec(),
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
            address_1_utxo_set.into_vec(None),
            vec![Utxo {
                outpoint: PublicOutPoint {
                    txid: tx.txid().to_vec(),
                    vout: 1
                },
                value: 400,
                height: 1
            }]
        );

        // Address 2 should receive 1500 Satoshi
        assert_eq!(
            address_2_utxo_set.into_vec(None),
            vec![Utxo {
                outpoint: PublicOutPoint {
                    txid: tx.txid().to_vec(),
                    vout: 0
                },
                value: 1500,
                height: 1
            }]
        );
    }
}
