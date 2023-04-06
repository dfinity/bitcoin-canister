//! A copy of memory.rs that is compatible with the new version of stable-structures.
//! Once the migration to the new stable-structures version is complete, this file will
//! fully replace memory.rs

use ic_stable_structures_new::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
#[cfg(feature = "file_memory")]
use ic_stable_structures_new::FileMemory;
#[cfg(not(feature = "file_memory"))]
use ic_stable_structures_new::{DefaultMemoryImpl, RestrictedMemory};
use std::cell::RefCell;

const SMALL_UTXOS: MemoryId = MemoryId::new(2);
const MEDIUM_UTXOS: MemoryId = MemoryId::new(3);
const BALANCES: MemoryId = MemoryId::new(4);
const BLOCK_HEADERS: MemoryId = MemoryId::new(5);
const BLOCK_HEIGHTS: MemoryId = MemoryId::new(6);

#[cfg(feature = "file_memory")]
type InnerMemory = FileMemory;

#[cfg(not(feature = "file_memory"))]
type InnerMemory = RestrictedMemory<DefaultMemoryImpl>;

pub type Memory = VirtualMemory<InnerMemory>;

#[cfg(feature = "file_memory")]
thread_local! {
    static MEMORY: RefCell<Option<InnerMemory>> = RefCell::new(None);

    static MEMORY_MANAGER: RefCell<Option<MemoryManager<InnerMemory>>> = RefCell::new(None);
}

#[cfg(not(feature = "file_memory"))]
thread_local! {
    static MEMORY: RefCell<Option<InnerMemory>> = RefCell::new(Some(RestrictedMemory::new(DefaultMemoryImpl::default(), 0..10000)));

    static MEMORY_MANAGER: RefCell<Option<MemoryManager<InnerMemory>>> =
        RefCell::new(Some(MemoryManager::init_with_bucket_size(MEMORY.with(|m| m.borrow().clone().unwrap()), 1024)));
}

fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<InnerMemory>) -> R) -> R {
    MEMORY_MANAGER.with(|cell| {
        f(cell
            .borrow()
            .as_ref()
            .expect("memory manager not initialized"))
    })
}

pub fn get_utxos_small_memory() -> Memory {
    with_memory_manager(|m| m.get(SMALL_UTXOS))
}

pub fn get_utxos_medium_memory() -> Memory {
    with_memory_manager(|m| m.get(MEDIUM_UTXOS))
}

pub fn get_balances_memory() -> Memory {
    with_memory_manager(|m| m.get(BALANCES))
}

pub fn get_block_headers_memory() -> Memory {
    with_memory_manager(|m| m.get(BLOCK_HEADERS))
}

pub fn get_block_heights_memory() -> Memory {
    with_memory_manager(|m| m.get(BLOCK_HEIGHTS))
}
