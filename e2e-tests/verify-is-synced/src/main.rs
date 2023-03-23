use bitcoin::{
    blockdata::constants::genesis_block, consensus::Encodable, Address, Block, BlockHeader,
    Network as BitcoinNetwork, OutPoint,
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
const ADDRESS_3: &str = "bcrt1qp045tvzkxx0292645rxem9eryc7jpwsk3dy60h";
const ADDRESS_4: &str = "bcrt1qjft8fhexv4znxu22hed7gxtpy2wazjn0x079mn";
const ADDRESS_5: &str = "bcrt1qenhfslne5vdqld0djs0h0tfw225tkkzzc60exh";

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
    static BLOCKS: RefCell<Vec<BlockBlob>> = RefCell::new(Vec::new());

    static COUNT: Cell<u64> = Cell::new(0);

    static BLOCK_HEADERS: RefCell<Vec<BlockHeaderBlob>> = RefCell::new(Vec::new());
}

// Initialize the blocks.
#[init]
fn init() {
    let network = BitcoinNetwork::Regtest;

    // Block 1: A single transaction that gives ADDRESS_1 50 BTC split over 10k inputs.
    let mut tx_1 = TransactionBuilder::new();
    for _ in 0..10_000 {
        tx_1 = tx_1.with_output(&Address::from_str(ADDRESS_1).unwrap(), 500_000);
    }
    let tx_1 = tx_1.build();
    let tx_1_id = tx_1.txid();

    let block_1 = BlockBuilder::with_prev_header(genesis_block(network).header)
        .with_transaction(tx_1)
        .build();
    append_block(&block_1);

    // Block 2: 10k transactions that transfer all of ADDRESS_1's BTC to ADDRESS_2
    let mut block_2_txs = vec![];
    for i in 0..10_000 {
        block_2_txs.push(
            TransactionBuilder::new()
                .with_input(OutPoint {
                    txid: tx_1_id,
                    vout: i,
                })
                .with_output(&Address::from_str(ADDRESS_2).unwrap(), 500_000)
                .build(),
        )
    }

    let mut block_2 = BlockBuilder::with_prev_header(block_1.header);
    for tx in block_2_txs.iter() {
        block_2 = block_2.with_transaction(tx.clone());
    }
    let block_2 = block_2.build();

    append_block(&block_2);

    // Remaining blocks contain a single coinbase transaction giving ADDRESS_3 some BTC.
    let block_3 = BlockBuilder::with_prev_header(block_2.header)
        .with_transaction(
            TransactionBuilder::new()
                .with_output(&Address::from_str(ADDRESS_3).unwrap(), 500_000)
                .build(),
        )
        .build();
    append_block(&block_3);

    let block_4 = BlockBuilder::with_prev_header(block_3.header)
        .with_transaction(
            TransactionBuilder::new()
                .with_output(&Address::from_str(ADDRESS_4).unwrap(), 500_000)
                .build(),
        )
        .build();
    append_block(&block_4);

    // Block 5: 10k transactions that transfer all of ADDRESS_2's BTC to ADDRESS_5
    let mut block_5_txs = vec![];
    for block_2_tx in block_2_txs {
        block_5_txs.push(
            TransactionBuilder::new()
                .with_input(OutPoint {
                    txid: block_2_tx.txid(),
                    vout: 0,
                })
                .with_output(&Address::from_str(ADDRESS_5).unwrap(), 500_000)
                .build(),
        )
    }

    let mut block_5 = BlockBuilder::with_prev_header(block_4.header);
    for tx in block_5_txs.into_iter() {
        block_5 = block_5.with_transaction(tx);
    }
    let block_5 = block_5.build();
    append_block(&block_5);

    let next_block_1 = BlockBuilder::with_prev_header(block_5.header)
        .with_transaction(
            TransactionBuilder::new()
                .with_output(&Address::from_str(ADDRESS_5).unwrap(), 500_000)
                .build(),
        )
        .build();
    append_block_header(&next_block_1.header);
    let next_block_2 = BlockBuilder::with_prev_header(next_block_1.header)
        .with_transaction(
            TransactionBuilder::new()
                .with_output(&Address::from_str(ADDRESS_5).unwrap(), 500_000)
                .build(),
        )
        .build();
    append_block_header(&next_block_2.header);
    let next_block_3 = BlockBuilder::with_prev_header(next_block_2.header)
        .with_transaction(
            TransactionBuilder::new()
                .with_output(&Address::from_str(ADDRESS_5).unwrap(), 500_000)
                .build(),
        )
        .build();
    append_block_header(&next_block_3.header);
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
            remaining_follow_ups: 2,
        })
    } else if count == 2 {
        // Send another part of block 2.
        GetSuccessorsResponse::FollowUp(BLOCKS.with(|b| b.borrow()[1].clone())[20..40].to_vec())
    } else if count == 3 {
        // Send rest of block 2.
        GetSuccessorsResponse::FollowUp(BLOCKS.with(|b| b.borrow()[1].clone())[40..].to_vec())
    } else if count == 4 {
        // Send block 3 in full.
        GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
            blocks: vec![BLOCKS.with(|b| b.borrow()[2].clone())],
            next: vec![],
        })
    } else if count == 5 {
        // Send block 4 in full.
        GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
            blocks: vec![BLOCKS.with(|b| b.borrow()[3].clone())],
            next: vec![],
        })
    } else if count == 6 {
        // Send block 5 in full, and all next block headers.
        GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
            blocks: vec![BLOCKS.with(|b| b.borrow()[4].clone())],
            next: BLOCK_HEADERS.with(|b| {
                vec![
                    b.borrow()[0].clone(),
                    b.borrow()[1].clone(),
                    b.borrow()[2].clone(),
                ]
            }),
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

fn append_block_header(block_header: &BlockHeader) {
    let mut block_bytes = vec![];
    block_header.consensus_encode(&mut block_bytes).unwrap();
    BLOCK_HEADERS.with(|b| b.borrow_mut().push(block_bytes));
}

fn main() {}
