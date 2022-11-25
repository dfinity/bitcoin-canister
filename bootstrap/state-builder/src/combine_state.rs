//! A script for combining the various computed states into a single file.
//!
//! Example run:
//!
//! cargo run --release --bin combine-state -- \
//!   --output canister.bin \
//!   --canister-state-dir ./canister_state
use clap::Parser;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager},
    FileMemory, Memory,
};
use std::{fs::File, path::PathBuf};

const WASM_PAGE_SIZE: u64 = 65536;

// The amount of data to read from a file in a single read request.
const CHUNK_SIZE: u64 = 1024 * WASM_PAGE_SIZE;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    canister_state_dir: PathBuf,

    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    output: PathBuf,
}

fn write_memory(memory_manager: &MemoryManager<FileMemory>, memory_id: u8, memory: &PathBuf) {
    println!("Writing memory {}..", memory_id);
    let dst = memory_manager.get(MemoryId::new(memory_id));

    let src = FileMemory::new(File::open(memory).unwrap());
    dst.grow(src.size());

    let src_size_in_bytes = src.size() * WASM_PAGE_SIZE;
    println!("Memory size: {}", src_size_in_bytes);

    // Read the file in small chunks.
    let mut bytes_read = 0;
    let mut buf = vec![0; CHUNK_SIZE as usize];
    while bytes_read + CHUNK_SIZE <= src_size_in_bytes {
        src.read(bytes_read, &mut buf);
        dst.write(bytes_read, &buf);

        bytes_read += buf.len() as u64;
        println!("Wrote {} bytes", bytes_read);
    }

    if src_size_in_bytes - bytes_read != 0 {
        // Read remaining bytes.
        let mut buf = vec![0; (src_size_in_bytes - bytes_read) as usize];
        src.read(bytes_read, &mut buf);
        dst.write(bytes_read, &buf);
        bytes_read += buf.len() as u64;
        assert_eq!(bytes_read, src_size_in_bytes);
        println!("Wrote {} bytes", bytes_read);
    }
}

fn main() {
    let args = Args::parse();

    // Create the file memory of the whole canister.
    let memory = FileMemory::new(File::create(&args.output).expect("Cannot create output file."));
    let memory_manager = MemoryManager::init(memory);

    // Add the various memories.
    let mut p = args.canister_state_dir.clone();
    p.push("./address_utxos");
    write_memory(&memory_manager, 1, &p);

    let mut p = args.canister_state_dir.clone();
    p.push("./small_utxos");
    write_memory(&memory_manager, 2, &p);

    let mut p = args.canister_state_dir.clone();
    p.push("./medium_utxos");
    write_memory(&memory_manager, 3, &p);

    let mut p = args.canister_state_dir;
    p.push("./balances");
    write_memory(&memory_manager, 4, &p);
}
