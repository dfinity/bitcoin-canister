//! A script for uploading a file into the stable memory of a canister.
//!
//! This script relies on the `uploader` canister to be able to upload the file.
//!
//! Example run:
//!
//! cargo run --example uploader-script -- \
//!     --canister-id rwlgt-iiaaa-aaaaa-aaaaa-cai \
//!     --state ./file-to-upload \
//!     --ic-network http://127.0.0.1:8000 \
//!     --fetch-root-key
use candid::{encode_args, CandidType, Decode, Encode};
use clap::Parser;
use ic_agent::{export::Principal, Agent};
use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::PathBuf,
};
use uploader::*;
use url::Url;

#[derive(Parser, Debug)]
struct Args {
    /// A path to load the state from.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    state: PathBuf,

    /// Url of the IC network to connect to.
    #[clap(long, default_value_t = Url::parse("https://ic0.app").unwrap())]
    ic_network: Url,

    /// Whether or not to fetch the root key. Should be true for testnets, false otherwise.
    #[clap(long, default_value_t = false)]
    fetch_root_key: bool,

    /// The canister to upload the state to.
    #[clap(long)]
    canister_id: Principal,

}

#[derive(CandidType)]
struct Empty;

// Helper method for uploading a chunk.
async fn upload(agent: &Agent, canister_id: &Principal, chunk_start: u64, bytes: &[u8]) {
    let waiter = garcon::Delay::builder()
        .throttle(std::time::Duration::from_millis(500))
        .timeout(std::time::Duration::from_secs(60 * 5))
        .build();

    agent
        .update(canister_id, "upload_chunk")
        .with_arg(encode_args((chunk_start, bytes.to_vec())).unwrap())
        .call_and_wait(waiter)
        .await
        .expect("uploading chunk failed");
}

#[async_std::main]
async fn main() {
    let args = Args::parse();

    // Connect to the given network.
    #[allow(deprecated)]
    let agent = Agent::builder()
        .with_url(args.ic_network.to_string())
        .build()
        .expect("agent creation must succeed");

    let f = File::open(args.state).expect("opening state file must succeed");

    // Fetch root key if needed.
    if args.fetch_root_key {
        agent
            .fetch_root_key()
            .await
            .expect("fetch root key must succeed");
    }

    // Fetch the indices of the missing chunks that need to be uploaded.
    let response: Vec<u8> = agent
        .query(&args.canister_id, "get_missing_chunk_indices")
        .with_arg(Encode!(&Empty).unwrap())
        .call()
        .await
        .unwrap();
    let missing_chunk_indices = Decode!(&response, Vec<u64>).unwrap();

    // Upload the missing chunks.
    let mut reader = BufReader::new(f);
    for chunk_index in missing_chunk_indices {
        let offset = chunk_index * PAGE_SIZE_IN_BYTES;
        let mut buf = vec![0; CHUNK_SIZE_IN_BYTES as usize];
        reader
            .seek(SeekFrom::Start(offset))
            .expect("seek must succeed");
        let bytes_read = reader.read(&mut buf).expect("read must succeed");
        if bytes_read == 0 {
            break;
        }

        println!("Uploading chunk at {}", chunk_index);
        upload(&agent, &args.canister_id, chunk_index, &buf[..bytes_read]).await;
    }
}
