//! A copy of memory.rs that is compatibly with the new version of stable-structures.
//! Once the migration to the new stable-structures version is complete, this file will
//! fully replace memory.rs

use ic_stable_structures_new::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
#[cfg(not(feature = "file_memory"))]
use ic_stable_structures_new::DefaultMemoryImpl;
#[cfg(feature = "file_memory")]
use ic_stable_structures_new::FileMemory;
use std::cell::RefCell;

const BLOCK_HEADERS: MemoryId = MemoryId::new(5);
const BLOCK_HEIGHTS: MemoryId = MemoryId::new(6);

#[cfg(feature = "file_memory")]
type InnerMemory = FileMemory;

#[cfg(not(feature = "file_memory"))]
type InnerMemory = DefaultMemoryImpl;

pub type Memory = VirtualMemory<InnerMemory>;

#[cfg(feature = "file_memory")]
thread_local! {
    static MEMORY: RefCell<Option<InnerMemory>> = RefCell::new(None);

    static MEMORY_MANAGER: RefCell<Option<MemoryManager<InnerMemory>>> = RefCell::new(None);
}

#[cfg(not(feature = "file_memory"))]
thread_local! {
    static MEMORY: RefCell<Option<InnerMemory>> = RefCell::new(Some(InnerMemory::default()));

    static MEMORY_MANAGER: RefCell<Option<MemoryManager<InnerMemory>>> =
        RefCell::new(Some(MemoryManager::init(MEMORY.with(|m| m.borrow().clone().unwrap()))));
}

fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<InnerMemory>) -> R) -> R {
    MEMORY_MANAGER.with(|cell| {
        f(cell
            .borrow()
            .as_ref()
            .expect("memory manager not initialized"))
    })
}

pub fn get_block_headers_memory() -> Memory {
    with_memory_manager(|m| m.get(BLOCK_HEADERS))
}

pub fn get_block_heights_memory() -> Memory {
    with_memory_manager(|m| m.get(BLOCK_HEIGHTS))
}
