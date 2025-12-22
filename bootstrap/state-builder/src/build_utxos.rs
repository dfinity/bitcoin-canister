//! A script for building the Bitcoin canister's UTXOs from a UTXO dump text file.
//!
//! Example run:
//!
//! cargo run --release --bin build-utxos --features file_memory -- \
//!   --network testnet \
//!   --output output-dir \
//!   --utxos-dump-path utxos-dump.csv
use bitcoin::{hashes::Hash, Address, Txid as BitcoinTxid};
use clap::Parser;
use ic_btc_canister::types::into_bitcoin_network;
use ic_btc_canister::{types::TxOut, with_state, with_state_mut, CanisterArg};
use ic_btc_interface::{Flag, InitConfig, Network};
use ic_btc_types::{OutPoint, Txid};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager},
    FileMemory, Memory,
};
use std::{
    fs::{create_dir_all, File},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    str::FromStr,
};

const WASM_PAGE_SIZE: u64 = 65536;

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
}

fn write_memory_to_file(path: &PathBuf, memory_id: MemoryId) {
    let canister_mem = ic_btc_canister::get_memory();
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

    // Set a temp file to use as the memory while we compute the state.
    // This is significantly slower than building the state in RAM, but now that state sizes
    // are very large, this is the more reliable approach to avoid OOM issues.
    ic_btc_canister::memory::set_memory(FileMemory::new(tempfile::tempfile().unwrap()));

    // Create the output directory if it doesn't already exist.
    create_dir_all(&args.output).unwrap();

    // Read the UTXOs from the UTXOs dump.
    let utxos_file = File::open(args.utxos_dump_path).unwrap();
    let reader = BufReader::new(utxos_file);

    ic_btc_canister::init(Some(CanisterArg::Init(InitConfig {
        network: Some(args.network),
        api_access: Some(Flag::Disabled),
        ..Default::default()
    })));

    with_state_mut(|s| {
        for (i, line) in reader.lines().enumerate() {
            let line = line.unwrap();
            let parts: Vec<_> = line.split(',').collect();

            let txid = Txid::from(
                BitcoinTxid::from_str(parts[1])
                    .unwrap()
                    .as_raw_hash()
                    .as_byte_array()
                    .to_vec(),
            );
            let vout: u32 = parts[2].parse().unwrap();
            let amount: u64 = parts[3].parse().unwrap();
            let script = parts[6];
            let height: u32 = parts[0].parse().unwrap();
            let address_str = parts[5];

            if i % 100_000 == 0 {
                println!("Processed {} UTXOs", i);
            }

            // Scripts in the chainstate database are compressed in the case of standard types.
            // Instead of using the scripts from the database, we can infer the script from the
            // address. Otherwise, we use the script in the chainstate database as-is.
            let script = match Address::from_str(address_str) {
                Ok(address) => address
                    .require_network(into_bitcoin_network(args.network))
                    .unwrap()
                    .script_pubkey()
                    .as_bytes()
                    .to_vec(),
                Err(_) => hex::decode(script).unwrap(),
            };

            // Insert the UTXO
            let outpoint = OutPoint { txid, vout };
            if !bitcoin::Script::from_bytes(&script).is_provably_unspendable() {
                let txout = TxOut {
                    value: amount,
                    script_pubkey: script,
                };

                let found = s.utxos.utxos.insert(outpoint, (txout, height));
                assert!(!found); // A UTXO cannot be seen more than once.
            }
        }
    });

    // Write the memories corresponding to the small and medium UTXOs.
    // These are stable structures so we write the memory as-is.
    println!("Writing small UTXOs...");
    let mut p = args.output.clone();
    p.push("small_utxos");
    write_memory_to_file(&p, MemoryId::new(2));

    println!("Writing medium UTXOs...");
    let mut p = args.output.clone();
    p.push("medium_utxos");
    write_memory_to_file(&p, MemoryId::new(3));

    // Write the large UTXOs, which is a standard BTreeMap so it needs to
    // be serialized.
    println!("Writing large UTXOs...");
    let mut p = args.output;
    p.push("large_utxos");

    let mut file = match File::create(&p) {
        Err(err) => panic!("couldn't create {}: {}", p.display(), err),
        Ok(file) => file,
    };

    with_state(|s| {
        let mut bytes = vec![];
        ciborium::ser::into_writer(&s.utxos.utxos.large_utxos, &mut bytes)
            .expect("failed to encode large utxos");
        match file.write_all(&bytes) {
            Err(err) => panic!("couldn't write to {}: {}", p.display(), err),
            Ok(_) => println!("successfully wrote to {}", p.display()),
        };
    });
}
