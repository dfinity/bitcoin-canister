//! A script for building the Bitcoin canister's address UTXOs from a UTXO dump text file.
//!
//! Example run:
//!
//! cargo run --release --bin build-address-utxos -- \
//!   --network testnet \
//!   --output balances.bin \
//!   --utxos-dump-path utxos-dump.csv
use bitcoin::{Address, Txid as BitcoinTxid};
use clap::Parser;
use ic_btc_canister::types::{Address as OurAddress, AddressUtxo, Network, OutPoint, Txid};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use std::{
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

    let memory = DefaultMemoryImpl::default();
    let mut address_utxos: StableBTreeMap<_, AddressUtxo, ()> =
        StableBTreeMap::init(memory.clone());

    for (i, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        let parts: Vec<_> = line.split(',').collect();

        let txid = Txid::from(BitcoinTxid::from_str(parts[1]).unwrap().to_vec());
        let vout: u32 = parts[2].parse().unwrap();
        let address_str = parts[5];
        let height: u32 = parts[9].parse().unwrap();

        if i % 100_000 == 0 {
            println!("Processed {} UTXOs", i);
        }

        if let Ok(address) = Address::from_str(address_str) {
            let address: OurAddress = address.into();

            address_utxos
                .insert(
                    AddressUtxo {
                        address,
                        height,
                        outpoint: OutPoint {
                            txid: txid.clone(),
                            vout,
                        },
                    },
                    (),
                )
                .unwrap();
        }
    }

    println!("Writing stable structure to file...");
    let mut file = match File::create(&args.output) {
        Err(err) => panic!("couldn't create {}: {}", args.output.display(), err),
        Ok(file) => file,
    };

    match file.write_all(&memory.borrow()) {
        Err(err) => panic!("couldn't write to {}: {}", args.output.display(), err),
        Ok(_) => println!("successfully wrote balances to {}", args.output.display()),
    };
}
