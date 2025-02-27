use candid::CandidType;
use ic_cdk_macros::{query, update};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

type BlockBlob = Vec<u8>;
type BlockHeaderBlob = Vec<u8>;
type BlockHash = Vec<u8>;

#[derive(CandidType, Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
enum Network {
    #[serde(rename = "mainnet")]
    Mainnet,
    #[serde(rename = "testnet")]
    Testnet,
    #[serde(rename = "regtest")]
    Regtest,
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SendTransactionInternalRequest {
    network: Network,
    transaction: Vec<u8>,
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
    static LAST_TRANSACTION: RefCell<Vec<u8>> = const { RefCell::new(Vec::new())};
}

#[update]
fn bitcoin_send_transaction_internal(request: SendTransactionInternalRequest) {
    LAST_TRANSACTION.with(|c| c.replace(request.transaction));
}

#[query]
fn get_last_transaction() -> Vec<u8> {
    LAST_TRANSACTION.with(|c| c.borrow().clone())
}

#[update]
fn bitcoin_get_successors(_request: GetSuccessorsRequest) -> GetSuccessorsResponse {
    // Empty response
    GetSuccessorsResponse::Complete(GetSuccessorsCompleteResponse {
        blocks: vec![],
        next: vec![],
    })
}

fn main() {}
