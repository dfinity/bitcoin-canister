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
use bitcoin::{consensus::Decodable, Address, BlockHash, BlockHeader, Txid};
use byteorder::{LittleEndian, ReadBytesExt};
use clap::Parser;
use ic_btc_canister::{
    heartbeat, memory, pre_upgrade, runtime,
    state::main_chain_height,
    types::{
        self, Address as OurAddress, Config, GetSuccessorsCompleteResponse, GetSuccessorsResponse,
        Network,
    },
    with_state, with_state_mut,
};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
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

fn main() {
    let args = Args::parse();

    // Read the UTXOs from the UTXOs dump.
    let utxos_file = File::open(args.utxos_dump_path).unwrap();
    let reader = BufReader::new(utxos_file);

    /*let map: StableBTreeMap<DefaultMemoryImpl, AddressUtxo, ()> = StableBTreeMap::init(
        DefaultMemoryImpl::default(),
        90 + 36, // max outpoint size.
        0,       // No values are stored in the map.
    );*/

    let balances_memory = DefaultMemoryImpl::default();
    let mut balances: StableBTreeMap<_, OurAddress, u64> =
        StableBTreeMap::init(balances_memory.clone(), 90, 8);

    let mut balances_fast: BTreeMap<OurAddress, u64> = BTreeMap::new();

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
        let height: u32 = parts[8].parse().unwrap();

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
                        .entry(address)
                        .and_modify(|curr| *curr += amount)
                        .or_insert(amount);
                    //let address_balance = balances_fast.get(&address).unwrap_or(0);
                    //balances_fast
                    //   .insert(address, address_balance + amount);
                    //.expect("insertion must succeed");
                }

                /*s.utxos
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
                .unwrap();*/
            }
            Err(_) => {}
        }

        // Insert the UTXO
        /*let outpoint = types::OutPoint { txid, vout };

        let txout = types::TxOut {
            value: amount,
            script_pubkey: script,
        };

        let x = s.utxos.utxos.insert(outpoint, (txout, height));
        assert!(!x); // not seen this utxo before.*/
    }

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

    println!("writing file");
    let mut balances_file = match File::create(&args.output) {
        Err(err) => panic!("couldn't create {}: {}", args.output.display(), err),
        Ok(file) => file,
    };

    match balances_file.write_all(&balances_memory.borrow()) {
        Err(err) => panic!("couldn't write to {}: {}", args.output.display(), err),
        Ok(_) => println!("successfully wrote balances to {}", args.output.display()),
    };
}
