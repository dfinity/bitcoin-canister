use crate::state::{UTXO_KEY_SIZE, UTXO_VALUE_MAX_SIZE_MEDIUM, UTXO_VALUE_MAX_SIZE_SMALL};
use crate::types::{OutPoint, Storable, TxOut};
use ic_btc_types::Height;
use stable_structures::{btreemap, DefaultMemoryImpl, Memory, RestrictedMemory, StableBTreeMap};
use std::collections::BTreeMap;

type CanisterMemory = RestrictedMemory<DefaultMemoryImpl>;

/// A key-value store for UTXOs (unspent transaction outputs).
///
/// A UTXO is the tuple (OutPoint, TxOut, Height). For ease of access, UTXOs are
/// stored such that the OutPoint is the key, and (TxOut, Height) is the value.
///
/// Ordinarily, a standard `BTreeMap` would suffice for storing UTXOs, but UTXOs
/// have properties that make storing them more complex.
///
///  * Number of entries: As of early 2022, there are tens of millions of UTXOs.
///    Storing them in a standard `BTreeMap` would make checkpointing very
///    inefficient as it would require serializing all the UTXOs. To work
///    around this, `StableBTreeMap` is used instead, where checkpointing grows
///    linearly only with the number of dirty memory pages.
///
///  * A `StableBTreeMap` allocates the maximum size possible for a key/value.
///    Scripts in Bitcoin are bounded to 10k bytes, but allocating 10k for every
///    UTXO wastes a lot of memory and increases the number of memory read/writes.
///
///    Based on a study of mainnet up to height ~705,000, the following is the
///    distribution of script sizes in UTXOs:
///
///    | Script Size           |  # UTXOs     | % of Total |
///    |-----------------------|--------------|------------|
///    | <= 25 bytes           |  74,136,585  |   98.57%   |
///    | > 25 && <= 201 bytes  |   1,074,004  |    1.43%   |
///    | > 201 bytes           |          13  | 0.00002%   |
///
///    Because of the skewness in the sizes of the script, the KV store for
///    UTXOs is split into buckets:
///
///    1) "Small" to store UTXOs with script size <= 25 bytes.
///    2) "Medium" to store UTXOs with script size > 25 bytes && <= 201 bytes.
///    3) "Large" to store UTXOs with script size > 201 bytes.
pub struct Utxos {
    // A map storing the UTXOs that are "small" in size.
    pub small_utxos: StableBTreeMap<CanisterMemory, Vec<u8>, Vec<u8>>,

    // A map storing the UTXOs that are "medium" in size.
    pub medium_utxos: StableBTreeMap<CanisterMemory, Vec<u8>, Vec<u8>>,

    // A map storing the UTXOs that are "large" in size.
    // The number of entries stored in this map is tiny (see docs above), so a
    // standard `BTreeMap` suffices.
    pub large_utxos: BTreeMap<OutPoint, (TxOut, Height)>,
}

impl Default for Utxos {
    fn default() -> Self {
        Self {
            small_utxos: StableBTreeMap::init(
                small_utxos_memory(),
                UTXO_KEY_SIZE,
                UTXO_VALUE_MAX_SIZE_SMALL,
            ),
            medium_utxos: StableBTreeMap::init(
                medium_utxos_memory(),
                UTXO_KEY_SIZE,
                UTXO_VALUE_MAX_SIZE_MEDIUM,
            ),
            large_utxos: BTreeMap::default(),
        }
    }
}

impl Utxos {
    /// Inserts a utxo into the map.
    /// Returns true if there was a previous value for the key in the map, false otherwise.
    pub fn insert(&mut self, key: OutPoint, value: (TxOut, Height)) -> bool {
        let value_encoded = value.to_bytes();

        if value_encoded.len() <= UTXO_VALUE_MAX_SIZE_SMALL as usize {
            self.small_utxos
                .insert(key.to_bytes(), value_encoded)
                .expect("Inserting small UTXO must succeed.")
                .is_some()
        } else if value_encoded.len() <= UTXO_VALUE_MAX_SIZE_MEDIUM as usize {
            self.medium_utxos
                .insert(key.to_bytes(), value_encoded)
                .expect("Inserting medium UTXO must succeed.")
                .is_some()
        } else {
            self.large_utxos.insert(key, value).is_some()
        }
    }

    /// Returns the value associated with the given outpoint if it exists.
    pub fn get(&self, key: &OutPoint) -> Option<(TxOut, Height)> {
        let key_vec = key.to_bytes();

        if let Some(value) = self.small_utxos.get(&key_vec) {
            return Some(<(TxOut, Height)>::from_bytes(value));
        }

        if let Some(value) = self.medium_utxos.get(&key_vec) {
            return Some(<(TxOut, Height)>::from_bytes(value));
        }

        self.large_utxos.get(key).cloned()
    }

    /// Removes a key from the map, returning the previous value at the key if it exists.
    pub fn remove(&mut self, key: &OutPoint) -> Option<(TxOut, Height)> {
        let key_vec = key.to_bytes();

        if let Some(value) = self.small_utxos.remove(&key_vec) {
            return Some(<(TxOut, Height)>::from_bytes(value));
        }

        if let Some(value) = self.medium_utxos.remove(&key_vec) {
            return Some(<(TxOut, Height)>::from_bytes(value));
        }

        self.large_utxos.remove(key)
    }

    /// Returns `true` if the key exists in the map, `false` otherwise.
    pub fn contains_key(&self, key: &OutPoint) -> bool {
        self.small_utxos.contains_key(&key.to_bytes())
            || self.medium_utxos.contains_key(&key.to_bytes())
            || self.large_utxos.contains_key(key)
    }

    /// Gets an iterator over the entries of the map.
    /// NOTE: The entries are not guaranteed to be sorted in any particular way.
    pub fn iter(&self) -> Iter<CanisterMemory> {
        Iter::new(self)
    }

    pub fn len(&self) -> u64 {
        self.large_utxos.len() as u64 + self.small_utxos.len() + self.medium_utxos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.large_utxos.is_empty() && self.small_utxos.is_empty() && self.medium_utxos.is_empty()
    }
}

/// An iterator over the entries in [`Utxos`].
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Iter<'a, M: Memory> {
    small_utxos_iter: btreemap::Iter<'a, M, Vec<u8>, Vec<u8>>,
    medium_utxos_iter: btreemap::Iter<'a, M, Vec<u8>, Vec<u8>>,
    large_utxos_iter: std::collections::btree_map::Iter<'a, OutPoint, (TxOut, Height)>,
}

impl<'a> Iter<'a, CanisterMemory> {
    fn new(utxos: &'a Utxos) -> Self {
        Self {
            small_utxos_iter: utxos.small_utxos.iter(),
            medium_utxos_iter: utxos.medium_utxos.iter(),
            large_utxos_iter: utxos.large_utxos.iter(),
        }
    }
}

impl<M: Memory + Clone> Iterator for Iter<'_, M> {
    type Item = (OutPoint, (TxOut, Height));

    fn next(&mut self) -> Option<Self::Item> {
        // First, iterate over the small utxos.
        if let Some((key_bytes, value_bytes)) = self.small_utxos_iter.next() {
            return Some((
                OutPoint::from_bytes(key_bytes),
                <(TxOut, Height)>::from_bytes(value_bytes),
            ));
        }

        // Second, iterate over the medium utxos.
        if let Some((key_bytes, value_bytes)) = self.medium_utxos_iter.next() {
            return Some((
                OutPoint::from_bytes(key_bytes),
                <(TxOut, Height)>::from_bytes(value_bytes),
            ));
        }

        // Finally, iterate over the large utxos.
        self.large_utxos_iter
            .next()
            .map(|(k, v)| (k.clone(), v.clone()))
    }
}

// Creates a memory region for the "small" UTXOs.
// The memory region currently is small for testing purposes and
// will be much larger in the future.
fn small_utxos_memory() -> CanisterMemory {
    RestrictedMemory::new(DefaultMemoryImpl::default(), 0..999)
}

// Creates a memory region for the "medium" UTXOs.
// The memory region currently is small for testing purposes and
// will be much larger in the future.
fn medium_utxos_memory() -> CanisterMemory {
    RestrictedMemory::new(DefaultMemoryImpl::default(), 1000..1999)
}
