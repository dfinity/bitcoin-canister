use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl, Memory as MemoryTrait,
};
use std::os::unix::fs::FileExt;
use std::{
    cell::RefCell,
    fs::{File, OpenOptions},
    rc::Rc,
    thread::LocalKey,
};

const WASM_PAGE_SIZE: u64 = 65536;

const UPGRADES: MemoryId = MemoryId::new(0);
const ADDRESS_OUTPOINTS: MemoryId = MemoryId::new(1);
const SMALL_UTXOS: MemoryId = MemoryId::new(2);
const MEDIUM_UTXOS: MemoryId = MemoryId::new(3);
const BALANCES: MemoryId = MemoryId::new(4);
const BLOCK_HEADERS: MemoryId = MemoryId::new(5);
const BLOCK_HEIGHTS: MemoryId = MemoryId::new(6);

pub type Memory = VirtualMemory<FileMemory>;

thread_local! {
    static MEMORY: FileMemory = FileMemory::new(std::path::Path::new("foo.dat")).unwrap();

    static MEMORY_MANAGER: MemoryManager<FileMemory>
        = MemoryManager::init(MEMORY.with(|m| m.clone()));
}

pub fn get_memory() -> &'static LocalKey<FileMemory> {
    &MEMORY
}

pub fn get_upgrades_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(UPGRADES))
}

pub fn get_address_utxos_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(ADDRESS_OUTPOINTS))
}

pub fn get_utxos_small_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(SMALL_UTXOS))
}

pub fn get_utxos_medium_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(MEDIUM_UTXOS))
}

pub fn get_balances_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(BALANCES))
}

pub fn get_block_headers_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(BLOCK_HEADERS))
}

pub fn get_block_heights_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.get(BLOCK_HEIGHTS))
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


use std::{
    path::Path,
};

#[derive(Clone)]
pub struct FileMemory(Rc<RefCell<FileMemoryInner>>);

struct FileMemoryInner(File);

impl FileMemory {
    pub fn new(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        Ok(Self(Rc::new(RefCell::new(FileMemoryInner(file)))))
    }
}

impl MemoryTrait for FileMemory {
    /// Returns the current size of the stable memory in WebAssembly
    /// pages. (One WebAssembly page is 64Ki bytes.)
    fn size(&self) -> u64 {
        let len = self.0.borrow().0.metadata().unwrap().len();
        assert_eq!(
            len % WASM_PAGE_SIZE,
            0,
            "File size must correspond to exact page sizes"
        );
        len / WASM_PAGE_SIZE
    }

    /// Tries to grow the memory by new_pages many pages containing
    /// zeroes.  If successful, returns the previous size of the
    /// memory (in pages).  Otherwise, returns -1.
    fn grow(&self, pages: u64) -> i64 {
        let previous_size = self.size();
        self.0
            .borrow()
            .0
            .set_len((previous_size + pages) * WASM_PAGE_SIZE)
            .expect("grow must succeed");
        previous_size as i64
    }

    /// Copies the data referred to by offset out of the stable memory
    /// and replaces the corresponding bytes in dst.
    fn read(&self, offset: u64, dst: &mut [u8]) {
        let bytes_read = self
            .0
            .borrow()
            .0
            .read_at(dst, offset)
            .expect("offset out of bounds");

        assert_eq!(bytes_read, dst.len(), "read out of bounds");
    }

    /// Copies the data referred to by src and replaces the
    /// corresponding segment starting at offset in the stable memory.
    fn write(&self, offset: u64, src: &[u8]) {
        let bytes_written = self
            .0
            .borrow()
            .0
            .write_at(src, offset)
            .expect("offset out of bounds");
        assert_eq!(bytes_written, src.len(), "write out of bounds");
    }
}
