use clap::Parser;
use rusty_leveldb::{LdbIterator, Options, DB};
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    /// A path to load/store the state.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    chain_state_path: PathBuf,
}

// Builds an index of where the blocks are present in the files.
fn main() {
    let args = Args::parse();

    // The path of the leveldb that contains the index.
    let mut db = DB::open(args.chain_state_path, Options::default()).unwrap();
    let mut iter = db.new_iter().unwrap();

    println!("iter next: {:?}", iter.next().unwrap());
    /*while (key, value) in
        block_index.insert(res.0, (res.1, res.2));
        blockhash = res.3;
    }

    block_index*/
}
