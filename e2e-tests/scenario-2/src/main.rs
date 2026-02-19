use bitcoin::{
    blockdata::constants::genesis_block, consensus::Encodable, Address, Block,
    Network as BitcoinNetwork,
};
use candid::CandidType;
use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};
use ic_cdk::{init, update};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::str::FromStr;

type BlockBlob = Vec<u8>;
type BlockHeaderBlob = Vec<u8>;
type BlockHash = Vec<u8>;

const ADDRESS: &str = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8";

#[derive(CandidType, Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
enum Network {
    #[serde(rename = "mainnet")]
    Mainnet,
    #[serde(rename = "testnet")]
    Testnet,
    #[serde(rename = "regtest")]
    Regtest,
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
enum GetSuccessorsRequest {
    #[serde(rename = "initial")]
    Initial(GetSuccessorsRequestInitial),
    #[serde(rename = "follow_up")]
    FollowUp(u8),
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct GetSuccessorsRequestInitial {
    pub network: Network,
    pub processed_block_hashes: Vec<BlockHash>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Hash, PartialEq, Eq, Serialize)]
enum GetSuccessorsResponse {
    #[serde(rename = "complete")]
    Complete(GetSuccessorsCompleteResponse),
    #[serde(rename = "partial")]
    Partial(GetSuccessorsPartialResponse),
    #[serde(rename = "follow_up")]
    FollowUp(BlockBlob),
}

#[derive(CandidType, Clone, Debug, Default, Deserialize, Hash, PartialEq, Eq, Serialize)]
struct GetSuccessorsCompleteResponse {
    blocks: Vec<BlockBlob>,
    next: Vec<BlockHeaderBlob>,
}

#[derive(CandidType, Clone, Debug, Default, Deserialize, Hash, PartialEq, Eq, Serialize)]
struct GetSuccessorsPartialResponse {
    partial_block: BlockBlob,
    next: Vec<BlockHeaderBlob>,
    remaining_follow_ups: u8,
}

thread_local! {
    static BLOCKS: RefCell<Vec<BlockBlob>> = const { RefCell::new(Vec::new())};

    static COUNT: Cell<u64> = const { Cell::new(0)};
}

// The number of blocks to generate (on top of genesis)
const NUM_BLOCKS: u32 = 4;

// The number of transactions in each of these blocks.
const TXS_PER_BLOCK: u32 = 10_000;

// Initialize the blocks.
#[init]
fn init() {
    let network = BitcoinNetwork::Regtest;

    // Generate NUM_BLOCKS blocks, each with NUM_TRANSACTIONS transactions.
    let mut prev_header = genesis_block(network).header;
    let mut lock_time_offset = 0;
    for _ in 0..NUM_BLOCKS {
        let mut block = BlockBuilder::with_prev_header(prev_header);
        for i in lock_time_offset..lock_time_offset + TXS_PER_BLOCK {
            // A transaction giving 1 satoshi to the address.
            block = block.with_transaction(
                TransactionBuilder::new()
                    .with_lock_time(i)
                    .with_output(&Address::from_str(ADDRESS).unwrap().assume_checked(), 1)
                    .build(),
            );
        }
        let block = block.build();
        append_block(&block);
        prev_header = block.header;
        lock_time_offset += TXS_PER_BLOCK;
    }
}

#[update]
fn bitcoin_get_successors(request: GetSuccessorsRequest) -> GetSuccessorsResponse {
    if let GetSuccessorsRequest::Initial(GetSuccessorsRequestInitial { network, .. }) = &request {
        assert_eq!(
            *network,
            Network::Regtest,
            "request must be set to the regtest network"
        );
    }

    let count = COUNT.with(|c| c.get()) as usize;

    let res = if count < BLOCKS.with(|b| b.borrow().len()) {
        GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
            blocks: vec![BLOCKS.with(|b| b.borrow()[count].clone())],
            next: vec![],
        })
    } else {
        // Empty response
        GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
            blocks: vec![],
            next: vec![],
        })
    };

    COUNT.with(|c| c.set(c.get() + 1));
    res
}

fn append_block(block: &Block) {
    let mut block_bytes = vec![];
    block.consensus_encode(&mut block_bytes).unwrap();
    BLOCKS.with(|b| b.borrow_mut().push(block_bytes));
}

fn main() {}
