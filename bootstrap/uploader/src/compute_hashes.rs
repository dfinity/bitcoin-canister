//! A script for computing the hashes of the chunks of a file.
//!
//! The size of the chunk is hard-coded in `CHUNK_SIZE_IN_BYTES`.
use clap::Parser;
use std::{fs::File, io::Read, path::PathBuf};
use uploader::*;

#[derive(Parser, Debug)]
struct Args {
    /// The path of the file to compute its hashes.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    file: PathBuf,
}

fn main() {
    let args = Args::parse();

    // Read file in chunks, and print the sha256 hash of each of these chunks.
    let mut file = File::open(args.file).expect("opening file must succeed");
    let mut chunk = vec![0; CHUNK_SIZE_IN_BYTES as usize];
    loop {
        let bytes_read = file.read(&mut chunk).unwrap();

        if bytes_read == 0 {
            break;
        }

        let hash = sha256::digest(&chunk[0..bytes_read]);
        println!("{}", hash);
    }
}
