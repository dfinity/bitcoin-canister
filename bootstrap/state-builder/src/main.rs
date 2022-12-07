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
    types::{Config, Flag, GetSuccessorsCompleteResponse, GetSuccessorsResponse, Network},
    with_state,
};
use rusty_leveldb::{Options, DB};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

type Height = u32;
type FileNumber = u32;
type FileOffset = u32;

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
    // The path of the leveldb that contains the index.
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

// Reads a block's info from leveldb.
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
    blk_file.read_exact(&mut block_bytes).unwrap();
    block_bytes
}

#[async_std::main]
async fn main() {
    let args = Args::parse();

    let tip = BlockHash::from_str(&args.tip).expect("tip must be valid.");

    println!("Building block index...");

    let block_index = build_block_index(&args.blocks_path, tip);

    println!("Initializing...");

    ic_btc_canister::init(Config {
        stability_threshold: 0,
        network: args.network,
        api_access: Flag::Disabled,
        ..Config::default()
    });

    let mut blocks_path = args.blocks_path.clone();
    blocks_path.push("blocks");

    for (height, (file, offset)) in block_index.into_iter() {
        let block_bytes = read_block(&blocks_path, file, offset);

        runtime::set_successors_response(runtime::GetSuccessorsReply::Ok(
            GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
                blocks: vec![block_bytes],
                next: vec![],
            }),
        ));

        // Run the heartbeat until we process all the blocks.
        while with_state(main_chain_height) != height {
            heartbeat().await;
        }

        println!("Height :{:?}", with_state(main_chain_height));
    }

    // Run the pre-upgrade hook to save all the state into the memory.
    pre_upgrade();

    println!(
        "memory size: {:?}",
        ic_btc_canister::get_memory().borrow().len()
    );

    let mut file = match File::create(&args.state_path) {
        Err(err) => panic!("couldn't create {}: {}", args.state_path.display(), err),
        Ok(file) => file,
    };

    match file.write_all(&ic_btc_canister::get_memory().borrow()) {
        Err(err) => panic!("couldn't write to {}: {}", args.state_path.display(), err),
        Ok(_) => println!("successfully wrote state to {}", args.state_path.display()),
    };
}
