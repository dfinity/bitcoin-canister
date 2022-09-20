use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl, Memory as MemoryTrait,
};

const WASM_PAGE_SIZE: u64 = 65536;

const UPGRADES: MemoryId = MemoryId::new(0);
const ADDRESS_OUTPOINTS: MemoryId = MemoryId::new(1);
const SMALL_UTXOS: MemoryId = MemoryId::new(2);
const MEDIUM_UTXOS: MemoryId = MemoryId::new(3);

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: MemoryManager<DefaultMemoryImpl>
        = MemoryManager::init(DefaultMemoryImpl::default());
}

pub fn get_upgrades_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(UPGRADES))
}

pub fn get_address_outpoints_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(ADDRESS_OUTPOINTS))
}

pub fn get_utxos_small_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(SMALL_UTXOS))
}

pub fn get_utxos_medium_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(MEDIUM_UTXOS))
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
