//! A script for building the Bitcoin canister's address UTXOs from a UTXO dump text file.
//!
//! Example run:
//!
//! cargo run --release --bin build-address-utxos -- \
//!   --network testnet \
//!   --output balances.bin \
//!   --utxos-dump-path utxos-dump.csv
use bitcoin::{Address as BitcoinAddress, ScriptBuf, Txid as BitcoinTxid};
use clap::Parser;
use ic_btc_canister::types::{into_bitcoin_network, Address, AddressUtxo};
use ic_btc_interface::Network;
use ic_btc_types::{OutPoint, Txid};
use ic_stable_structures::{
    storable::Blob, BoundedStorable, DefaultMemoryImpl, StableBTreeMap, Storable,
};
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
    let mut address_utxos: StableBTreeMap<Blob<{ AddressUtxo::MAX_SIZE as usize }>, (), _> =
        StableBTreeMap::init(memory.clone());

    for (i, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        let parts: Vec<_> = line.split(',').collect();

        let txid = Txid::from(BitcoinTxid::from_str(parts[1]).unwrap().as_ref());
        let vout: u32 = parts[2].parse().unwrap();
        let address_str = parts[5];
        let height: u32 = parts[0].parse().unwrap();
        let script = parts[6];

        if i % 100_000 == 0 {
            println!("Processed {} UTXOs", i);
        }

        // Load the address. The UTXO dump tool we use doesn't output all the addresses
        // we support, so if parsing the address itself fails, we try parsing the script directly.
        let address = BitcoinAddress::from_str(address_str)
            .map(|address| address.assume_checked())
            .or_else(|_| {
                BitcoinAddress::from_script(
                    &ScriptBuf::from(hex::decode(script).expect("script must be valid hex")),
                    into_bitcoin_network(args.network),
                )
            });

        if let Ok(address) = address {
            let address: Address = address.into();

            address_utxos
                .insert(
                    Blob::try_from(
                        AddressUtxo {
                            address,
                            height,
                            outpoint: OutPoint {
                                txid: txid.clone(),
                                vout,
                            },
                        }
                        .to_bytes()
                        .as_ref(),
                    )
                    .unwrap(),
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
