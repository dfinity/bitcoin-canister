//! A script for building the Bitcoin canister's state and storing it into a file.
//!
//! Example run:
//!
//! cargo run --bin state-builder --release -- \
//!     --state-path data.bin \
//!     --network testnet \
//!     --blocks-path /path/to/data/testnet3 \
//!     --tip 000000002ce019cc4a8f2af62b3ecf7c30a19d29828b25268a0194dbac3cac50
use bitcoin::{consensus::Decodable, BlockHash, BlockHeader};
use byteorder::{LittleEndian, ReadBytesExt};
use clap::Parser;
use ic_btc_canister::{
    heartbeat, pre_upgrade, runtime,
    state::main_chain_height,
    types::{GetSuccessorsCompleteResponse, GetSuccessorsResponse, Network},
    with_state,
};
use ic_stable_structures::Memory;
use rusty_leveldb::{Options, DB};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
    str::FromStr,
};

type Height = u32;
type FileNumber = u32;
type FileOffset = u32;

const WASM_PAGE_SIZE: u64 = 65536;

struct FileMemory(File);

impl Memory for FileMemory {
    /// Returns the current size of the stable memory in WebAssembly
    /// pages. (One WebAssembly page is 64Ki bytes.)
    fn size(&self) -> u64 {
        let len = self.0.metadata().unwrap().len();
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
            .set_len(pages * WASM_PAGE_SIZE)
            .expect("grow must succeed");
        previous_size as i64
    }

    /// Copies the data referred to by offset out of the stable memory
    /// and replaces the corresponding bytes in dst.
    fn read(&self, offset: u64, dst: &mut [u8]) {
        let bytes_read = self.0.read_at(dst, offset).expect("offset out of bounds");

        assert_eq!(bytes_read, dst.len(), "read out of bounds");
    }

    /// Copies the data referred to by src and replaces the
    /// corresponding segment starting at offset in the stable memory.
    fn write(&self, offset: u64, src: &[u8]) {
        let bytes_written = self.0.write_at(src, offset).expect("offset out of bounds");
        assert_eq!(bytes_written, src.len(), "write out of bounds");
    }
}

#[derive(Parser, Debug)]
struct Args {
    /// A path to load/store the state.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    state_path: PathBuf,

    /// The path to the `datadir` of `bitcoind`.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    blocks_path: PathBuf,

    /// The bitcoin network.
    #[clap(long)]
    network: Network,

    /// The hash of the tip of the chain to build.
    #[clap(long)]
    tip: String,
}

// How to read Bitcoin's varint format.
trait VarIntRead: std::io::Read {
    fn read_varint(&mut self) -> usize {
        let mut n = 0;
        loop {
            let ch_data = self.read_u8();
            n = (n << 7) | (ch_data & 0x7F) as usize;
            if ch_data & 0x80 > 0 {
                n += 1;
            } else {
                break;
            }
        }
        n
    }

    fn read_u8(&mut self) -> u8 {
        let mut slice = [0u8; 1];
        self.read_exact(&mut slice).unwrap();
        slice[0]
    }
}

impl VarIntRead for Cursor<&[u8]> {}
impl VarIntRead for Cursor<Vec<u8>> {}

// Builds an index of where the blocks are present in the files.
fn build_block_index(path: &Path, tip: BlockHash) -> BTreeMap<Height, (FileNumber, FileOffset)> {
    // The path of the levelsdb that contains the index.
    let mut block_index_path = path.to_path_buf();
    block_index_path.push("blocks");
    block_index_path.push("index");

    // Build the index. We start from the given tip and go all the way back to genesis.
    let mut block_index: BTreeMap<Height, (FileNumber, FileOffset)> = BTreeMap::new();
    let mut blockhash = tip;
    let mut db = DB::open(block_index_path, Options::default()).unwrap();
    while let Some(res) = get_block_info(&mut db, &blockhash) {
        block_index.insert(res.0, (res.1, res.2));
        blockhash = res.3;
    }

    block_index
}

// Reads a block's info from levelsDB.
fn get_block_info(
    db: &mut DB,
    block_hash: &BlockHash,
) -> Option<(Height, FileNumber, FileOffset, BlockHash)> {
    let mut key: Vec<u8> = vec![98];
    key.extend(block_hash.to_vec());

    let value = db.get(&key).unwrap();

    let mut reader = Cursor::new(value);

    let _version = reader.read_varint() as i32;
    let height = reader.read_varint() as u32;
    let _status = reader.read_varint() as u32;

    let _tx = reader.read_varint() as u32;
    let file = reader.read_varint() as i32;
    let offset = reader.read_varint() as u32;
    let _undo_pos = reader.read_varint() as u32;

    match BlockHeader::consensus_decode(&mut reader) {
        Err(_) => None,
        Ok(header) => Some((height, file as u32, offset, header.prev_blockhash)),
    }
}

fn read_block(block_path: &Path, file: u32, offset: u32) -> Vec<u8> {
    let mut blk_file = File::open(block_path.join(format!("blk{:0>5}.dat", file))).unwrap();

    // Read the block size, which is the 4 bytes just before the offset where the block starts.
    blk_file.seek(SeekFrom::Start((offset - 4) as u64)).unwrap();
    let block_size = blk_file.read_u32::<LittleEndian>().unwrap();

    let mut block_bytes = vec![0; block_size as usize];
    blk_file.read(&mut block_bytes).unwrap();
    block_bytes
}

#[async_std::main]
async fn main() {
    let args = Args::parse();

    let tip = BlockHash::from_str(&args.tip).expect("tip must be valid.");

    let memory_size = ic_btc_canister::get_memory().with(|m| m.size());
    if memory_size == 0 {
        println!("Initializing new state...");
        ic_btc_canister::init(ic_btc_canister::types::InitPayload {
            stability_threshold: 0,
            network: args.network,
            blocks_source: None,
        });
    } else {
        println!("Loading existing state...");
        ic_btc_canister::post_upgrade();
    }

    println!("Building block index...");
    let block_index = build_block_index(&args.blocks_path, tip);

    /*ctrlc::set_handler(move || {
        // Run the pre-upgrade hook to save all the state into the memory.
        println!("Running pre-upgrade...");
        pre_upgrade();

        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");*/


    println!("state height: {}", with_state(main_chain_height));

    let mut blocks_path = args.blocks_path.clone();
    blocks_path.push("blocks");

    for (height, (file, offset)) in block_index.into_iter() {
        if height < with_state(main_chain_height) {
            continue;
        }

        let block_bytes = read_block(&blocks_path, file, offset);

        runtime::set_successors_response(GetSuccessorsResponse::Complete(
            GetSuccessorsCompleteResponse {
                blocks: vec![block_bytes],
                next: vec![],
            },
        ));

        // Run the heartbeat until we process all the blocks.
        while with_state(main_chain_height) != height {
            heartbeat().await;
        }
    }

    // Run the pre-upgrade hook to save all the state into the memory.
    println!("Running pre-upgrade...");
    pre_upgrade();

    println!(
        "memory size: {:?}",
        ic_btc_canister::get_memory().with(|m| m.size())
    );

    /*let mut file = match File::create(&args.state_path) {
        Err(err) => panic!("couldn't create {}: {}", args.state_path.display(), err),
        Ok(file) => file,
    };

    ic_btc_canister::get_memory().with(|m| match file.write_all(&m.borrow()) {
        Err(err) => panic!("couldn't write to {}: {}", args.state_path.display(), err),
        Ok(_) => println!("successfully wrote state to {}", args.state_path.display()),
    });*/
}
