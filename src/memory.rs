//! The purpose of this code is to make the memory being used in unit tests and
//! in production functionally equivalent.
//!
//! In production, a `DefaultMemoryImpl` resolves to the `Ic0StableMemory`,
//! which is a thin wrapper around the stable memory API.
//!
//! In unit tests, a `DefaultMemoryImpl` resolves to the `VectorMemory`, which
//! simulates a stable memory using a `Vec`.
//!
//! There's a subtle difference between the two environments: the stable memory
//! in production is a singleton, while the vector memories created in unit
//! tests are not. This difference can cause unit tests to fail that would
//! otherwise succeed in production and vice versa.
//!
//! This code removes this distinction and makes the two environments
//! consistent, making the memory being used a singleton in both production and
//! in unit tests.
use stable_structures::DefaultMemoryImpl;

thread_local! {
    static MEMORY: DefaultMemoryImpl = DefaultMemoryImpl::default();
}

/// Returns an instance of the memory.
/// The memory is a singleton that is cheaply cloned.
pub fn get() -> DefaultMemoryImpl {
    MEMORY.with(|m| m.clone())
}
