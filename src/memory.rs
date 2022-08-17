use stable_structures::{DefaultMemoryImpl, RestrictedMemory};
use std::ops::Range;

// The purpose of this singleton is to make the memory being used in unit tests
// and in production functionally equivalent.
//
// In production, a `DefaultMemoryImpl` resolves to the `Ic0StableMemory`,
// which is a thin wrapper around the stable memory API.
//
// In unit tests, a `DefaultMemoryImpl` resolves to the `VectorMemory`, which
// simulates a stable memory using a `Vec`.
//
// There's a subtle difference between the two environments: the stable memory
// in production is a singleton, while the vector memories created in unit
// tests are not. This difference can cause unit tests to fail that would
// otherwise succeed in production and vice versa.
//
// This singleton removes this distinction and makes the two environments
// consistent, making the memory being used a singleton in both production and
// in unit tests.
thread_local! {
    static MEMORY: DefaultMemoryImpl = DefaultMemoryImpl::default();
}

// A memory used for storing bits of the state during updates.
const UPGRADES_MEMORY: Range<u64> = 0..1_000;

// Memories for stable structures.
// NOTE: The sizes specified below are for testing purposes and are
//       insufficient for production.
const ADDRESS_OUTPOINTS: Range<u64> = 1_000..2_000;
const UTXOS_SMALL: Range<u64> = 2_000..3_000;
const UTXOS_MEDIUM: Range<u64> = 4_000..5_000;

pub fn get_upgrades_memory() -> RestrictedMemory<DefaultMemoryImpl> {
    RestrictedMemory::new(get_memory(), UPGRADES_MEMORY)
}

pub fn get_address_outpoints_memory() -> RestrictedMemory<DefaultMemoryImpl> {
    RestrictedMemory::new(get_memory(), ADDRESS_OUTPOINTS)
}

pub fn get_utxos_small_memory() -> RestrictedMemory<DefaultMemoryImpl> {
    RestrictedMemory::new(get_memory(), UTXOS_SMALL)
}

pub fn get_utxos_medium_memory() -> RestrictedMemory<DefaultMemoryImpl> {
    RestrictedMemory::new(get_memory(), UTXOS_MEDIUM)
}

// Returns an instance of the memory.
// The memory is a singleton that is cheaply cloned.
fn get_memory() -> DefaultMemoryImpl {
    MEMORY.with(|m| m.clone())
}
