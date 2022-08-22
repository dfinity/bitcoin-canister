use candid::CandidType;
use ic_cdk_macros::update;
use serde::Deserialize;

type BlockBlob = Vec<u8>;
type BlockHeaderBlob = Vec<u8>;

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
struct GetSuccessorsRequest {
    anchor: Vec<u8>,
    processed_block_hashes: Vec<Vec<u8>>,
}

#[derive(CandidType, Clone, Debug, Default, Hash, PartialEq, Eq, Deserialize)]
struct GetSuccessorsResponse {
    blocks: Vec<BlockBlob>,
    next: Vec<BlockHeaderBlob>,
}

const BLOCK: &[u8] = &[
    0, 0, 0, 32, 6, 34, 110, 70, 17, 26, 11, 89, 202, 175, 18, 96, 67, 235, 91, 191, 40, 195, 79,
    58, 94, 51, 42, 31, 199, 178, 183, 60, 241, 136, 145, 15, 85, 62, 67, 249, 230, 181, 156, 95,
    185, 45, 16, 164, 161, 63, 188, 213, 202, 179, 233, 36, 217, 153, 78, 126, 15, 160, 146, 211,
    241, 7, 68, 110, 188, 170, 255, 98, 255, 255, 127, 32, 0, 0, 0, 0, 1, 2, 0, 0, 0, 0, 1, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    255, 255, 255, 255, 3, 81, 1, 1, 255, 255, 255, 255, 2, 0, 242, 5, 42, 1, 0, 0, 0, 22, 0, 20,
    69, 112, 201, 197, 244, 204, 35, 99, 203, 69, 51, 244, 178, 221, 53, 101, 8, 106, 236, 205, 0,
    0, 0, 0, 0, 0, 0, 0, 38, 106, 36, 170, 33, 169, 237, 226, 246, 28, 63, 113, 209, 222, 253, 63,
    169, 153, 223, 163, 105, 83, 117, 92, 105, 6, 137, 121, 153, 98, 180, 139, 235, 216, 54, 151,
    78, 140, 249, 1, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[update]
fn bitcoin_get_successors(_request: GetSuccessorsRequest) -> GetSuccessorsResponse {
    GetSuccessorsResponse {
        blocks: vec![BLOCK.to_vec()],
        next: vec![],
    }
}

fn main() {}
