use bitcoin::{
    blockdata::constants::genesis_block, consensus::Encodable, Address, Block,
    Network as BitcoinNetwork,
};
use candid::CandidType;
use ic_btc_test_utils::{BlockBuilder, TransactionBuilder};
use ic_cdk_macros::{init, update};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::str::FromStr;

type BlockBlob = Vec<u8>;
type BlockHeaderBlob = Vec<u8>;
type BlockHash = Vec<u8>;

const ADDRESS_1: &str = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8";
const ADDRESS_2: &str = "bcrt1qxp8ercrmfxlu0s543najcj6fe6267j97tv7rgf";

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
    num_pages: u8,
}

thread_local! {
    static BLOCKS: RefCell<Vec<BlockBlob>> = RefCell::new(Vec::new());

    static COUNT: Cell<u64> = Cell::new(0);
}

// Initialize the blocks.
#[init]
fn init() {
    let network = BitcoinNetwork::Regtest;

    // A transaction that gives ADDRESS_1 50 BTC split over many inputs.
    let mut tx = TransactionBuilder::new();
    for _ in 0..10_000 {
        tx = tx.with_output(&Address::from_str(ADDRESS_1).unwrap(), 500_000);
    }
    let tx = tx.build();

    let block_1 = BlockBuilder::with_prev_header(genesis_block(network).header)
        .with_transaction(tx)
        .build();

    append_block(&block_1);

    let block_2 = BlockBuilder::with_prev_header(block_1.header)
        .with_transaction(
            TransactionBuilder::new()
                .with_output(&Address::from_str(ADDRESS_2).unwrap(), 50_0000_0000)
                .build(),
        )
        .build();

    append_block(&block_2);

    // Add some extra blocks.
    let block_3 = BlockBuilder::with_prev_header(block_2.header)
        .with_transaction(
            TransactionBuilder::new()
                .with_output(&Address::from_str(ADDRESS_1).unwrap(), 500_000)
                .build(),
        )
        .build();
    append_block(&block_3);

    let block_4 = BlockBuilder::with_prev_header(block_3.header)
        .with_transaction(
            TransactionBuilder::new()
                .with_output(&Address::from_str(ADDRESS_1).unwrap(), 500_000)
                .build(),
        )
        .build();
    append_block(&block_4);
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

    let count = COUNT.with(|c| c.get());

    let res = if count == 0 {
        // Send block 1 in full.
        GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
            blocks: vec![BLOCKS.with(|b| b.borrow()[0].clone())],
            next: vec![],
        })
    } else if count == 1 {
        // Send part of block 2.
        GetSuccessorsResponse::Partial(GetSuccessorsPartialResponse {
            partial_block: BLOCKS.with(|b| b.borrow()[1].clone())[0..20].to_vec(),
            next: vec![],
            num_pages: 3,
        })
    } else if count == 2 {
        // Send another part of block 2.
        GetSuccessorsResponse::FollowUp(BLOCKS.with(|b| b.borrow()[1].clone())[20..40].to_vec())
    } else if count == 3 {
        // Send rest of block 2.
        GetSuccessorsResponse::FollowUp(BLOCKS.with(|b| b.borrow()[1].clone())[40..].to_vec())
    } else if count == 4 {
        // Send block 1 in full.
        GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
            blocks: vec![BLOCKS.with(|b| b.borrow()[2].clone())],
            next: vec![],
        })
    } else if count == 5 {
        // Send block 1 in full.
        GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
            blocks: vec![BLOCKS.with(|b| b.borrow()[3].clone())],
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
