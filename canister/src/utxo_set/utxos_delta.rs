use crate::types::{Address, OutPoint, TxOut};
use ic_btc_types::Height;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    iter::Iterator,
};

/// Tracks changes in the UTXO set that are made by a block.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq, Default)]
pub struct UtxosDelta {
    // Outpoints that have been added, accessible by address.
    added_outpoints: BTreeMap<Address, BTreeSet<OutPoint>>,

    // Outpoints that have been removed, accessible by address.
    removed_outpoints: BTreeMap<Address, BTreeSet<OutPoint>>,

    // A map of all the added outpoints and their addresses. The data here is identical to
    // `added_outpoints`, but is maintained additionally for performance reasons.
    all_added_outpoints: BTreeMap<OutPoint, Address>,

    // A set of all the removed outpoints. The data here is identical to `removed_outpoints`, but
    // is maintained additionally for performance reasons.
    all_removed_outpoints: BTreeSet<OutPoint>,

    // UTXOs that are added/removed.
    utxos: BTreeMap<OutPoint, (TxOut, Height)>,
}

impl UtxosDelta {
    /// Inserts a UTXO for the given address.
    pub fn insert(&mut self, address: Address, outpoint: OutPoint, tx_out: TxOut, height: Height) {
        self.added_outpoints
            .entry(address.clone())
            .or_insert(BTreeSet::new())
            .insert(outpoint.clone());

        self.all_added_outpoints.insert(outpoint.clone(), address);

        let res = self.utxos.insert(outpoint, (tx_out, height));
        assert_eq!(res, None, "Cannot add the same UTXO twice into UtxosDelta");
    }

    /// Removes a UTXO from the given address.
    pub fn remove(&mut self, address: Address, outpoint: OutPoint, tx_out: TxOut, height: Height) {
        // Was this UTXO already added? This can be the case if the ingesting block adds a UTXO,
        // then removes it. In this case, removing it is equivalent to deleting its addition from
        // the `UtxosDelta`.
        if let Some(address) = self.all_added_outpoints.remove(&outpoint) {
            // Remove it from the `utxos` map.
            let res = self.utxos.remove(&outpoint);
            assert!(res.is_some());

            // Remove it from the `added_outpoints` map.
            let res = self
                .added_outpoints
                .get_mut(&address)
                .expect("utxos of address must exist")
                .remove(&outpoint);
            assert!(res);

            return;
        }

        self.removed_outpoints
            .entry(address)
            .or_insert(BTreeSet::new())
            .insert(outpoint.clone());

        self.all_removed_outpoints.insert(outpoint.clone());

        let res = self.utxos.insert(outpoint, (tx_out, height));
        assert_eq!(res, None, "Cannot add the same UTXO twice into UtxosDelta");
    }

    pub fn get_added_outpoints(&self, address: &Address) -> BTreeSet<&OutPoint> {
        self.added_outpoints
            .get(address)
            .map(|t| t.iter().collect::<BTreeSet<_>>())
            .unwrap_or_default()
    }

    pub fn get_removed_outpoints(&self, address: &Address) -> BTreeSet<&OutPoint> {
        self.removed_outpoints
            .get(address)
            .map(|t| t.iter().collect::<BTreeSet<_>>())
            .unwrap_or_default()
    }

    pub fn is_outpoint_added(&self, outpoint: &OutPoint) -> bool {
        self.all_added_outpoints.contains_key(outpoint)
    }

    pub fn is_outpoint_removed(&self, outpoint: &OutPoint) -> bool {
        self.all_removed_outpoints.contains(outpoint)
    }

    pub fn get_utxo(&self, outpoint: &OutPoint) -> Option<&(TxOut, Height)> {
        self.utxos.get(outpoint)
    }
}
