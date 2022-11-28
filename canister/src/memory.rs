#[cfg(not(feature = "file_memory"))]
use ic_stable_structures::DefaultMemoryImpl;
#[cfg(feature = "file_memory")]
use ic_stable_structures::FileMemory;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    Memory as MemoryTrait,
};
use std::cell::RefCell;

const WASM_PAGE_SIZE: u64 = 65536;

const UPGRADES: MemoryId = MemoryId::new(0);
const ADDRESS_OUTPOINTS: MemoryId = MemoryId::new(1);
const SMALL_UTXOS: MemoryId = MemoryId::new(2);
const MEDIUM_UTXOS: MemoryId = MemoryId::new(3);
const BALANCES: MemoryId = MemoryId::new(4);
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

pub fn with_memory_manager_mut<R>(f: impl FnOnce(&mut MemoryManager<InnerMemory>) -> R) -> R {
    MEMORY_MANAGER.with(|cell| {
        f(cell
            .borrow_mut()
            .as_mut()
            .expect("memory manager not initialized"))
    })
}

pub fn get_memory() -> InnerMemory {
    MEMORY.with(|m| m.borrow().clone().expect("memory not initialized"))
}

pub fn set_memory(memory: InnerMemory) {
    MEMORY.with(|m| m.replace(Some(memory.clone())));
    MEMORY_MANAGER.with(|memory_manager| memory_manager.replace(Some(MemoryManager::init(memory))));
}

pub fn get_upgrades_memory() -> Memory {
    with_memory_manager(|m| m.get(UPGRADES))
}

pub fn get_address_utxos_memory() -> Memory {
    with_memory_manager(|m| m.get(ADDRESS_OUTPOINTS))
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

/// Writes the bytes at the specified offset, growing the memory size if needed.
pub fn write<M: MemoryTrait>(memory: &M, offset: u64, bytes: &[u8]) {
    let last_byte = offset
        .checked_add(bytes.len() as u64)
        .expect("Address space overflow");

    let size_pages = memory.size();
    let size_bytes = size_pages
        .checked_mul(WASM_PAGE_SIZE)
        .expect("Address space overflow");

    if size_bytes < last_byte {
        let diff_bytes = last_byte - size_bytes;
        let diff_pages = diff_bytes
            .checked_add(WASM_PAGE_SIZE - 1)
            .expect("Address space overflow")
            / WASM_PAGE_SIZE;
        if memory.grow(diff_pages) == -1 {
            panic!(
                "Failed to grow memory from {} pages to {} pages (delta = {} pages).",
                size_pages,
                size_pages + diff_pages,
                diff_pages
            );
        }
    }
    memory.write(offset, bytes);
}
