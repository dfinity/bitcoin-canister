//! A script for building the Bitcoin canister's state and storing it into a file.
//!
//! Example run:
//!
//! FIXME
//! cargo run --bin state-builder --release -- \
//!     --state-path data.bin \
//!     --network testnet \
//!     --blocks-path /path/to/data/testnet3 \
//!     --tip 000000002ce019cc4a8f2af62b3ecf7c30a19d29828b25268a0194dbac3cac50
use bitcoin::{consensus::Decodable, Address, Block as BitcoinBlock, BlockHash, BlockHeader, Txid};
use byteorder::{LittleEndian, ReadBytesExt};
use clap::Parser;
use ic_btc_canister::{
    heartbeat, memory, pre_upgrade, runtime,
    state::main_chain_height,
    types::{
        self, Address as OurAddress, Block, Config, GetSuccessorsCompleteResponse,
        GetSuccessorsResponse, Network,
    },
    unstable_blocks, with_state, with_state_mut, UnstableBlocks,
};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager},
    DefaultMemoryImpl, Memory, StableBTreeMap,
};
use rusty_leveldb::{Options, DB};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Parser, Debug)]
struct Args {
    /// The path of the UTXOs dump.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    utxos_dump_path: PathBuf,

    /// The path to store the state in.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    output: PathBuf,

    /// The bitcoin network.
    #[clap(long)]
    network: Network,
}

const WASM_PAGE_SIZE: u64 = 65536;

const BLOCK_100002: &str = "020000002840bc6c31378c0a314609fb50f21811c5370f7df387b30d109d620000000000a9858cc9be942ea7459f026b09e3c25287706bc3d0d9ba2d59d8ea39168c6ce72400065227f1001c4a0c98870201000000010000000000000000000000000000000000000000000000000000000000000000ffffffff3703a28601000427f1001c043b520100522cfabe6d6d0000000000000000000068692066726f6d20706f6f6c7365727665726aac1eeeed88ffffffff0100f2052a010000001976a914912e2b234f941f30b18afbb4fa46171214bf66c888ac000000000100000001c422ec82824d97c2894905ab8fcb73dbc0e16ee44797e1e1967db42cd9564218010000006c493046022100f18c97457e00c491d3eed5d9c2c5da33398595adf2708a07f677fb1e3eeeccba022100dc5c886192a9af7a28ab7689e766f3be6b01b61a4c675c97e8d2c99cd8b9d1320121037928262812eb9e73b9ca8039f8023db84b0a86c5caf6bc28cefb85e9943684acffffffff02a530ed10000000001976a91405e18e90cf803e17b9fa70abd2ad931389cc2cd488acd533591c000000001976a9148f3441dd22b15a30dcde56f9b3de7a61b7a3a74088ac00000000";

const BLOCK_100001: &str = "020000001e0a16bbadccde1d80c66597b1939e45f91b570d29f95fc158299e000000000041aa0dbf100d7c35d424e7829e8f9ced52d04fd1669d45637f4fc820ad315a4554ff055227f1001c9acbb5cc0101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0d03a186010144062f503253482fffffffff0100f2052a01000000232103202fa513e1f9e57f235d442849eb73d743a5b8b9f546d0727fcc410ad91031ccac00000000";

fn write_mem_to_file(path: &PathBuf, memory_id: MemoryId) {
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

    // Read the UTXOs from the UTXOs dump.
    let utxos_file = File::open(args.utxos_dump_path).unwrap();
    let reader = BufReader::new(utxos_file);

    ic_btc_canister::init(Config {
        network: args.network,
        ..Config::default()
    });

    /*let map: StableBTreeMap<DefaultMemoryImpl, AddressUtxo, ()> = StableBTreeMap::init(
        DefaultMemoryImpl::default(),
        90 + 36, // max outpoint size.
        0,       // No values are stored in the map.
    );*/

    let balances_memory = DefaultMemoryImpl::default();
    let mut balances: StableBTreeMap<_, OurAddress, u64> =
        StableBTreeMap::init(balances_memory.clone(), 90, 8);

    let mut balances_fast: BTreeMap<OurAddress, u64> = BTreeMap::new();

    with_state_mut(|s| {
        let mut first = true;
        for (i, line) in reader.lines().enumerate() {
            if first {
                // skip headers
                first = false;
                continue;
            }

            let line = line.unwrap();
            let parts: Vec<_> = line.split(',').collect();

            let txid = Txid::from_str(parts[1]).unwrap().to_vec();
            let vout: u32 = parts[2].parse().unwrap();
            let amount: u64 = parts[3].parse().unwrap();
            let address_str = parts[5];
            let script = parts[6];
            let height: u32 = parts[9].parse().unwrap();

            let mut script = hex::decode(script).unwrap();

            if i % 100_000 == 0 {
                println!("Processed {}", i);
            }

            let txid = types::Txid::from(txid);
            match Address::from_str(parts[5]) {
                Ok(address) => {
                    //println!("script pubkey: {:?}", address.script_pubkey().as_bytes());
                    //println!("script pubkey: {:?}", address.script_pubkey());
                    //panic!();

                    script = address.script_pubkey().as_bytes().to_vec();

                    let address: OurAddress = address.into();

                    // Update the balance of the address.
                    if amount != 0 {
                        balances_fast
                            .entry(address.clone())
                            .and_modify(|curr| *curr += amount)
                            .or_insert(amount);

                        //let address_balance = balances_fast.get(&address).unwrap_or(0);
                        //balances_fast
                        //   .insert(address, address_balance + amount);
                        //.expect("insertion must succeed");
                    }

                    s.utxos
                        .address_utxos
                        .insert(
                            types::AddressUtxo {
                                address,
                                height,
                                outpoint: types::OutPoint {
                                    txid: txid.clone(),
                                    vout,
                                },
                            },
                            (),
                        )
                        .unwrap();
                }
                Err(_) => {}
            }

            // Insert the UTXO
            let outpoint = types::OutPoint { txid, vout };

            if !bitcoin::Script::from(script.clone()).is_provably_unspendable() {
                let txout = types::TxOut {
                    value: amount,
                    script_pubkey: script,
                };

                let x = s.utxos.utxos.insert(outpoint, (txout, height));
                assert!(!x); // not seen this utxo before.
            }
        }
    });

    println!("converting to vec");
    let mut balances_fast: Vec<_> = balances_fast.into_iter().collect();

    /*println!("randomizing");
    use rand::prelude::*;
    use rand_chacha::ChaCha8Rng;

    let mut rng = ChaCha8Rng::seed_from_u64(1);

    balances_fast.shuffle(&mut rng);*/

    println!("writing");
    // Write the balances into a stable btreemap.
    for (address, amount) in balances_fast.into_iter() {
        if amount == 0 {
            println!("found zero!!");
        } else {
            balances.insert(address, amount);
        }
    }

    //   println!("running pre upgrade");
    // Run the pre-upgrade hook to save all the state into the memory.
    //    pre_upgrade();

    let mut p = args.output.clone();
    p.push("address_utxos");
    write_mem_to_file(&p, MemoryId::new(1));

    let mut p = args.output.clone();
    p.push("small_utxos");
    write_mem_to_file(&p, MemoryId::new(2));
    let mut p = args.output.clone();
    p.push("medium_utxos");
    write_mem_to_file(&p, MemoryId::new(3));

    /*println!(
        "memory size: {:?}",
        ic_btc_canister::get_memory().with(|m| m.borrow().len())
    );

    let mut file = match File::create(&args.output) {
        Err(err) => panic!("couldn't create {}: {}", args.output.display(), err),
        Ok(file) => file,
    };

    ic_btc_canister::get_memory().with(|m| match file.write_all(&m.borrow()) {
        Err(err) => panic!("couldn't write to {}: {}", args.output.display(), err),
        Ok(_) => println!("successfully wrote state to {}", args.output.display()),
    });*/

    println!("writing balances");
    let mut p = args.output.clone();
    p.push("balances");
    let mut balances_file = match File::create(&p) {
        Err(err) => panic!("couldn't create {}: {}", args.output.display(), err),
        Ok(file) => file,
    };

    match balances_file.write_all(&balances_memory.borrow()) {
        Err(err) => panic!("couldn't write to {}: {}", args.output.display(), err),
        Ok(_) => println!("successfully wrote balances to {}", args.output.display()),
    };

    println!("writing address utxos");

    // Insert unstable blocks.
    let x = hex::decode(BLOCK_100002).unwrap();
    let new_block = Block::new(BitcoinBlock::consensus_decode(x.as_slice()).unwrap());

    let y = hex::decode(BLOCK_100001).unwrap();
    let root_block = Block::new(BitcoinBlock::consensus_decode(y.as_slice()).unwrap());

    println!("root block hash {}", root_block.block_hash().to_string());
    println!("new block hash {}", new_block.block_hash().to_string());
    with_state_mut(|s| {
        s.unstable_blocks = UnstableBlocks::new(&s.utxos, 0, root_block);
        unstable_blocks::push(&mut s.unstable_blocks, &s.utxos, new_block).unwrap();
        s.utxos.next_height = 100_001;
    });

    // TODO: we also need to include the large UTXOs.

    pre_upgrade();
    let mut p = args.output.clone();
    p.push("upgrade");
    write_mem_to_file(&p, MemoryId::new(0));
}
