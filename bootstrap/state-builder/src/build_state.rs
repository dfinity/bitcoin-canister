//! A script for building the Bitcoin canister's UTXOs from a UTXO dump text file.
//!
//! Example run:
//!
//! cargo run --release --bin build-utxos -- \
//!   --network testnet \
//!   --output output-dir \
//!   --utxos-dump-path utxos-dump.csv
use bitcoin::{consensus::Decodable, Address, Block as BitcoinBlock, Txid as BitcoinTxid};
use clap::Parser;
use ic_btc_canister::{
    pre_upgrade,
    state::State,
    types::{Block, Config, Network, OutPoint, TxOut, Txid},
    unstable_blocks::{self, UnstableBlocks},
    with_state, with_state_mut,
};
use ic_btc_types::Height;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager},
    FileMemory, Memory, StableBTreeMap,
};
use std::{
    collections::BTreeMap,
    fs::{create_dir_all, File},
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    str::FromStr,
};

const WASM_PAGE_SIZE: u64 = 65536;

const BLOCK_100002: &str = "020000002840bc6c31378c0a314609fb50f21811c5370f7df387b30d109d620000000000a9858cc9be942ea7459f026b09e3c25287706bc3d0d9ba2d59d8ea39168c6ce72400065227f1001c4a0c98870201000000010000000000000000000000000000000000000000000000000000000000000000ffffffff3703a28601000427f1001c043b520100522cfabe6d6d0000000000000000000068692066726f6d20706f6f6c7365727665726aac1eeeed88ffffffff0100f2052a010000001976a914912e2b234f941f30b18afbb4fa46171214bf66c888ac000000000100000001c422ec82824d97c2894905ab8fcb73dbc0e16ee44797e1e1967db42cd9564218010000006c493046022100f18c97457e00c491d3eed5d9c2c5da33398595adf2708a07f677fb1e3eeeccba022100dc5c886192a9af7a28ab7689e766f3be6b01b61a4c675c97e8d2c99cd8b9d1320121037928262812eb9e73b9ca8039f8023db84b0a86c5caf6bc28cefb85e9943684acffffffff02a530ed10000000001976a91405e18e90cf803e17b9fa70abd2ad931389cc2cd488acd533591c000000001976a9148f3441dd22b15a30dcde56f9b3de7a61b7a3a74088ac00000000";

const BLOCK_100001: &str = "020000001e0a16bbadccde1d80c66597b1939e45f91b570d29f95fc158299e000000000041aa0dbf100d7c35d424e7829e8f9ced52d04fd1669d45637f4fc820ad315a4554ff055227f1001c9acbb5cc0101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0d03a186010144062f503253482fffffffff0100f2052a01000000232103202fa513e1f9e57f235d442849eb73d743a5b8b9f546d0727fcc410ad91031ccac00000000";

#[derive(Parser, Debug)]
struct Args {
    /// The path of the UTXOs dump.
    #[clap(long, value_hint = clap::ValueHint::FilePath)]
    utxos_dump_path: PathBuf,

    /// The directory to store the output in.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    output: PathBuf,

    /// The bitcoin network.
    #[clap(long)]
    network: Network,

    /// The bitcoin network.
    #[clap(long)]
    height: u32,
}

fn write_memory_to_file(path: &PathBuf, memory_id: MemoryId) {
    let canister_mem = ic_btc_canister::get_memory().with(|m| m.clone());
    let memory_manager = MemoryManager::init(canister_mem);

    let memory = memory_manager.get(memory_id);

    let mut memory_vec = vec![0; (memory.size() * WASM_PAGE_SIZE).try_into().unwrap()];

    memory.read(0, &mut memory_vec);

    let mut file = match File::create(path) {
        Err(err) => panic!("couldn't create {}: {}", path.display(), err),
        Ok(file) => file,
    };

    match file.write_all(&memory_vec) {
        Err(err) => panic!("couldn't write to {}: {}", path.display(), err),
        Ok(_) => println!("successfully wrote to {}", path.display()),
    };
}

fn main() {
    let args = Args::parse();

    // Create the output directory if it doesn't already exist.
    create_dir_all(&args.output).unwrap();

    // Read the UTXOs from the UTXOs dump.
    let utxos_file = File::open(args.utxos_dump_path).unwrap();
    let reader = BufReader::new(utxos_file);

    ic_btc_canister::init(Config {
        network: args.network,
        ..Config::default()
    });

    // Load large UTXOs.
    let mut p = args.output.clone();
    p.push("large_utxos");
    println!("reading large utxos");
    let mut bytes = vec![];
    File::open(p).unwrap().read_to_end(&mut bytes).unwrap();

    println!("done");

    // Load small and medium utxos.
    let mut p = args.output.clone();
    p.push("small_utxos");
    let small_utxos_mem = FileMemory::new(File::open(p).unwrap());
    let small_utxos: StableBTreeMap<_, Vec<u8>, Vec<u8>> =
        StableBTreeMap::init(small_utxos_mem, 0, 0);

    let mut p = args.output.clone();
    p.push("medium_utxos");
    let medium_utxos_mem = FileMemory::new(File::open(p).unwrap());
    let medium_utxos: StableBTreeMap<_, Vec<u8>, Vec<u8>> =
        StableBTreeMap::init(medium_utxos_mem, 0, 0);

    with_state_mut(|s| {
    });

    println!("done");

    let large_utxos: BTreeMap<OutPoint, (TxOut, Height)> =
        ciborium::de::from_reader(&*bytes).expect("failed to decode state");

    println!("found {} large utxos", large_utxos.len());

    // Insert unstable blocks.
    let x = hex::decode(BLOCK_100002).unwrap();
    let new_block = Block::new(BitcoinBlock::consensus_decode(x.as_slice()).unwrap());

    let y = hex::decode(BLOCK_100001).unwrap();
    let root_block = Block::new(BitcoinBlock::consensus_decode(y.as_slice()).unwrap());

    println!("root block hash: {:?}", root_block.block_hash());
    with_state_mut(|s| {
        s.utxos.next_height = args.height;

        s.utxos.utxos.large_utxos = large_utxos;
        s.utxos.utxos.small_utxos = small_utxos;
        s.utxos.utxos.medium_utxos = medium_utxos;

        // Ingest the blocks.
        s.unstable_blocks = UnstableBlocks::new(&s.utxos, 0, root_block);
        unstable_blocks::push(&mut s.unstable_blocks, &s.utxos, new_block).unwrap();
    });

    pre_upgrade();

    let mut p = args.output;
    p.push("state");
    write_memory_to_file(&p, MemoryId::new(0));
}
