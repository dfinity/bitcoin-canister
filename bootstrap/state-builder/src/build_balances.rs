//! A script for building the Bitcoin canister's balances from a UTXO dump text file.
//!
//! Example run:
//!
//! cargo run --release --bin build-balances -- \
//!   --network testnet \
//!   --output balances.bin \
//!   --utxos-dump-path utxos-dump.csv
use bitcoin::Address as BitcoinAddress;
use clap::Parser;
use ic_btc_canister::types::{Address, Network};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    str::FromStr,
};

#[derive(Parser, Debug)]
struct Args {
    /// The path of the UTXOs dump.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    utxos_dump_path: PathBuf,

    /// The path to store the output in.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    output: PathBuf,

    /// The bitcoin network.
    #[clap(long)]
    network: Network,
}

fn main() {
    let args = Args::parse();

    // Read the UTXOs from the UTXOs dump.
    let utxos_file = File::open(args.utxos_dump_path).unwrap();
    let reader = BufReader::new(utxos_file);

    // Compute the balances. We use a standard BTreeMap here for speed.
    let mut balances: BTreeMap<Address, u64> = BTreeMap::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        let parts: Vec<_> = line.split(',').collect();

        let amount: u64 = parts[3].parse().unwrap();
        let address_str = parts[5];

        if i % 100_000 == 0 {
            println!("Processed {} UTXOS", i);
        }

        if let Ok(address) = BitcoinAddress::from_str(address_str) {
            let address: Address = address.into();

            // Update the balance of the address.
            if amount != 0 {
                balances
                    .entry(address.clone())
                    .and_modify(|curr| *curr += amount)
                    .or_insert(amount);
            }
        }
    }

    // Shuffle the balances. Based on anecdotal evidence, inserting the elements in a random
    // order is ~40% more space efficient than inserting the elements in sorted order.
    println!("Shuffling...");
    let mut balances: Vec<_> = balances.into_iter().collect();
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    balances.shuffle(&mut rng);

    println!("Writing to stable structure...");
    let memory = DefaultMemoryImpl::default();
    let mut stable_balances: StableBTreeMap<_, Address, u64> =
        StableBTreeMap::init(memory.clone(), 90, 8);

    // Write the balances into a stable btreemap.
    for (address, amount) in balances.into_iter() {
        stable_balances.insert(address, amount).unwrap();
    }

    println!("Writing stable structure to file...");
    let mut balances_file = match File::create(&args.output) {
        Err(err) => panic!("couldn't create {}: {}", args.output.display(), err),
        Ok(file) => file,
    };

    match balances_file.write_all(&memory.borrow()) {
        Err(err) => panic!("couldn't write to {}: {}", args.output.display(), err),
        Ok(_) => println!("successfully wrote balances to {}", args.output.display()),
    };
}
