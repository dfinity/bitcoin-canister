use bitcoin::consensus::Encodable;
use ic_btc_interface::{GetBlockHeadersError, GetBlockHeadersRequest, GetBlockHeadersResponse};

use crate::{
    runtime::{performance_counter, print},
    state::main_chain_height,
    with_state, with_state_mut,
};

// The maximum number of block headers that are allowed to be included in a single
// `GetBlockHeadersResponse`.
const MAX_BLOCK_HEADERS_PER_RESPONSE: u32 = 100;

// Various profiling stats for tracking the performance of `get_block_headers`.
#[derive(Default, Debug)]
struct Stats {
    // The total number of instructions used to process the request.
    ins_total: u64,

    // The number of instructions used to build the block headers vec from stable blocks.
    ins_build_block_headers_stable_blocks: u64,

    // The number of instructions used to build the block headers vec from unstable blocks.
    ins_build_block_headers_unstable_blocks: u64,
}

fn verify_and_return_effective_range(
    request: &GetBlockHeadersRequest,
) -> Result<(u32, u32), GetBlockHeadersError> {
    let chain_height = with_state(main_chain_height);

    if request.start_height > chain_height {
        return Err(GetBlockHeadersError::StartHeightDoesNotExist {
            requested: request.start_height,
            chain_height,
        });
    }

    let (effective_start_height, mut effective_end_height) =
        if let Some(end_height) = request.end_height {
            if end_height < request.start_height {
                return Err(GetBlockHeadersError::StartHeightLargerThanEndHeight {
                    start_height: request.start_height,
                    end_height,
                });
            }

            if end_height > chain_height {
                return Err(GetBlockHeadersError::EndHeightDoesNotExist {
                    requested: end_height,
                    chain_height,
                });
            }
            // If `end_height` is provided then it should be the
            // end of effective height range.
            (request.start_height, end_height)
        } else {
            // If `end_height` is not provided then the end of effective
            // range should be the last block of the chain.
            (request.start_height, chain_height)
        };

    // Bound the length of block headers vec.
    effective_end_height = std::cmp::min(
        effective_end_height,
        effective_start_height + MAX_BLOCK_HEADERS_PER_RESPONSE - 1,
    );

    Ok((effective_start_height, effective_end_height))
}

fn get_block_headers_internal(
    request: &GetBlockHeadersRequest,
) -> Result<(GetBlockHeadersResponse, Stats), GetBlockHeadersError> {
    let (start_height, end_height) = verify_and_return_effective_range(request)?;

    let mut stats: Stats = Stats::default();

    // Build block headers vec.
    let ins_start = performance_counter();

    // Add requested block headers located in stable_blocks.
    let mut vec_headers: Vec<Vec<u8>> = with_state(|s| {
        s.stable_block_headers
            .get_block_headers_in_range(std::ops::RangeInclusive::new(start_height, end_height))
            .map(|header_blob| header_blob.into())
            .collect()
    });

    let ins_after_stable_blocks = performance_counter();

    stats.ins_build_block_headers_stable_blocks = ins_after_stable_blocks - ins_start;

    // Add requested block headers located in unstable_blocks.
    with_state(|s| {
        let unstable_block_headers = &mut s
            .unstable_blocks
            .get_block_headers_in_range(
                s.stable_height(),
                std::ops::RangeInclusive::new(start_height, end_height),
            )
            // Serialize headers of unstable blocks.
            .map(|header| {
                let mut serialized_header = vec![];
                header.consensus_encode(&mut serialized_header).unwrap();
                serialized_header
            })
            .collect();

        vec_headers.append(unstable_block_headers)
    });

    stats.ins_build_block_headers_unstable_blocks = performance_counter() - ins_after_stable_blocks;
    stats.ins_total = performance_counter();

    Ok((
        GetBlockHeadersResponse {
            tip_height: end_height,
            block_headers: vec_headers,
        },
        stats,
    ))
}

/// Given a start height and an optional end height from request,
/// the function returns the block headers in the provided range.
/// The range is inclusive, i.e., the block headers at the start
/// and end heights are returned as well.

/// If no end height is specified, all blocks until the tip height,
/// i.e., the largest available height, are returned. However, if
/// the range from the start height to the end height or the tip
/// height is large, only a prefix of the requested block headers
/// may be returned in order to bound the size of the response.
pub fn get_block_headers(
    request: GetBlockHeadersRequest,
) -> Result<GetBlockHeadersResponse, GetBlockHeadersError> {
    let (res, stats) = get_block_headers_internal(&request)?;

    // Observe metrics.
    with_state_mut(|s| {
        s.metrics.get_block_headers_total.observe(stats.ins_total);

        s.metrics
            .get_block_headers_stable_blocks
            .observe(stats.ins_build_block_headers_stable_blocks);

        s.metrics
            .get_block_headers_unstable_blocks
            .observe(stats.ins_build_block_headers_unstable_blocks);
    });

    // Print the number of instructions it took to process this request.
    print(&format!("[INSTRUCTION COUNT] {:?}: {:?}", request, stats));
    Ok(res)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        genesis_block,
        state::{self, ingest_stable_blocks_into_utxoset, insert_block},
        test_utils::BlockBuilder,
        with_state_mut,
    };
    use bitcoin::consensus::Encodable;
    use ic_btc_interface::{InitConfig, Network};
    use proptest::prelude::*;

    fn get_block_headers_helper() {
        let network = Network::Regtest;
        crate::init(InitConfig {
            stability_threshold: Some(1),
            network: Some(network),
            ..Default::default()
        });

        let block1 = BlockBuilder::with_prev_header(genesis_block(network).header()).build();
        let block2 = BlockBuilder::with_prev_header(block1.clone().header()).build();

        // Insert the blocks.
        // Genesis block and block1 should be stable, while block2 should be unstable.
        with_state_mut(|state| {
            insert_block(state, block1).unwrap();
            insert_block(state, block2).unwrap();
            ingest_stable_blocks_into_utxoset(state);
        });
    }

    #[test]
    fn get_block_headers_malformed_heights() {
        get_block_headers_helper();

        let start_height = 1;
        let end_height = 0;

        let err = get_block_headers(GetBlockHeadersRequest {
            start_height,
            end_height: Some(end_height),
        })
        .unwrap_err();

        assert_eq!(
            err,
            GetBlockHeadersError::StartHeightLargerThanEndHeight {
                start_height,
                end_height,
            }
        );
    }

    #[test]
    fn start_height_does_not_exist() {
        get_block_headers_helper();

        let start_height: u32 = 3;

        let err = get_block_headers(GetBlockHeadersRequest {
            start_height,
            end_height: None,
        })
        .unwrap_err();

        assert_eq!(
            err,
            GetBlockHeadersError::StartHeightDoesNotExist {
                requested: start_height,
                chain_height: 2
            }
        );
    }

    #[test]
    fn end_height_does_not_exist() {
        get_block_headers_helper();

        let start_height: u32 = 1;
        let end_height: u32 = 4;

        let err = get_block_headers(GetBlockHeadersRequest {
            start_height,
            end_height: Some(end_height),
        })
        .unwrap_err();

        assert_eq!(
            err,
            GetBlockHeadersError::EndHeightDoesNotExist {
                requested: end_height,
                chain_height: 2
            }
        );
    }

    #[test]
    fn genesis_block_only() {
        let network = Network::Regtest;
        crate::init(InitConfig {
            stability_threshold: Some(1),
            network: Some(network),
            ..Default::default()
        });

        let mut genesis_header_blob = vec![];
        genesis_block(network)
            .header()
            .consensus_encode(&mut genesis_header_blob)
            .unwrap();

        // We request all block headers starting from height 0, until the end of the chain.
        let response: GetBlockHeadersResponse = get_block_headers(GetBlockHeadersRequest {
            start_height: 0,
            end_height: None,
        })
        .unwrap();

        // The result should contain the header of the genesis block since it is the only block in the chain.
        assert_eq!(
            response,
            GetBlockHeadersResponse {
                tip_height: 0,
                block_headers: vec![genesis_header_blob.clone()]
            }
        );

        // We request a block at height 0.
        let response: GetBlockHeadersResponse = get_block_headers(GetBlockHeadersRequest {
            start_height: 0,
            end_height: Some(0),
        })
        .unwrap();

        // The result should contain the header of the genesis block.
        assert_eq!(
            response,
            GetBlockHeadersResponse {
                tip_height: 0,
                block_headers: vec![genesis_header_blob]
            }
        );
    }

    #[test]
    fn single_block() {
        let network = Network::Regtest;
        crate::init(InitConfig {
            stability_threshold: Some(1),
            network: Some(network),
            ..Default::default()
        });

        let block = BlockBuilder::with_prev_header(genesis_block(network).header()).build();

        // Insert the block.
        with_state_mut(|state| {
            state::insert_block(state, block.clone()).unwrap();
        });

        let mut genesis_header_blob = vec![];
        genesis_block(network)
            .header()
            .consensus_encode(&mut genesis_header_blob)
            .unwrap();

        // The response should contain the header of the genesis block.
        assert_eq!(
            get_block_headers(GetBlockHeadersRequest {
                start_height: 0,
                end_height: Some(0),
            })
            .unwrap(),
            GetBlockHeadersResponse {
                tip_height: 0,
                block_headers: vec![genesis_header_blob.clone()]
            }
        );

        let mut block_header_blob = vec![];
        block
            .header()
            .consensus_encode(&mut block_header_blob)
            .unwrap();

        // The response should contain the header of `block`.
        assert_eq!(
            get_block_headers(GetBlockHeadersRequest {
                start_height: 1,
                end_height: Some(1),
            })
            .unwrap(),
            GetBlockHeadersResponse {
                tip_height: 1,
                block_headers: vec![block_header_blob.clone()]
            }
        );

        // The response should contain the header of `block`.
        assert_eq!(
            get_block_headers(GetBlockHeadersRequest {
                start_height: 1,
                end_height: None,
            })
            .unwrap(),
            GetBlockHeadersResponse {
                tip_height: 1,
                block_headers: vec![block_header_blob.clone()]
            }
        );

        // The response should contain headers of all blocks.
        assert_eq!(
            get_block_headers(GetBlockHeadersRequest {
                start_height: 0,
                end_height: Some(1),
            })
            .unwrap(),
            GetBlockHeadersResponse {
                tip_height: 1,
                block_headers: vec![genesis_header_blob.clone(), block_header_blob.clone()]
            }
        );

        // The response should contain headers of all blocks.
        assert_eq!(
            get_block_headers(GetBlockHeadersRequest {
                start_height: 0,
                end_height: None,
            })
            .unwrap(),
            GetBlockHeadersResponse {
                tip_height: 1,
                block_headers: vec![genesis_header_blob.clone(), block_header_blob.clone()]
            }
        );
    }

    fn helper_initialize_and_get_header_blobs(
        stability_threshold: u128,
        block_num: u32,
        network: Network,
    ) -> Vec<Vec<u8>> {
        crate::init(InitConfig {
            stability_threshold: Some(stability_threshold),
            network: Some(network),
            ..Default::default()
        });
        let genesis_block = genesis_block(network);

        let mut prev_block_header = *genesis_block.header();
        let mut genesis_header_blob = vec![];
        genesis_block
            .header()
            .consensus_encode(&mut genesis_header_blob)
            .unwrap();

        let mut blobs = vec![genesis_header_blob];

        // Genesis block is already added hence we need to add `block_num - 1` more blocks.
        for _ in 0..block_num - 1 {
            let block = BlockBuilder::with_prev_header(&prev_block_header).build();
            prev_block_header = *block.header();

            let mut block_blob = vec![];
            block.header().consensus_encode(&mut block_blob).unwrap();
            blobs.push(block_blob);

            with_state_mut(|state| insert_block(state, block).unwrap());
        }

        with_state_mut(ingest_stable_blocks_into_utxoset);

        blobs
    }

    fn check_response(
        blobs: &[Vec<u8>],
        start_height: u32,
        end_height: Option<u32>,
        total_num_blocks: u32,
    ) {
        let response: GetBlockHeadersResponse = get_block_headers(GetBlockHeadersRequest {
            start_height,
            end_height,
        })
        .unwrap();

        // If the requested `end_height` is `None`, the tip should be the last block.
        // The Length of block headers vec in response should be bounded by `MAX_BLOCK_HEADERS_PER_RESPONSE`.
        let tip_height = end_height
            .unwrap_or(total_num_blocks - 1)
            .min(start_height + MAX_BLOCK_HEADERS_PER_RESPONSE - 1);

        assert_eq!(
            response,
            GetBlockHeadersResponse {
                tip_height,
                block_headers: blobs[start_height as usize..=tip_height as usize].into()
            }
        );
    }

    fn test_all_valid_combination_or_height_range(blobs: &[Vec<u8>], block_num: u32) {
        for start_height in 0..block_num {
            let mut end_height_range: Vec<Option<u32>> =
                (start_height..block_num).map(Some).collect::<Vec<_>>();
            end_height_range.push(None);
            for end_height in end_height_range {
                check_response(blobs, start_height, end_height, block_num);
            }
        }
    }

    #[test]
    fn get_block_headers_chain_10_blocks_all_combinations() {
        let stability_threshold = 3;
        let block_num: u32 = 10;
        let network = Network::Regtest;

        let blobs: Vec<Vec<u8>> =
            helper_initialize_and_get_header_blobs(stability_threshold, block_num, network);

        test_all_valid_combination_or_height_range(&blobs, block_num);
    }

    #[test]
    fn get_block_headers_test_max_block_headers_per_response() {
        let stability_threshold = 3;
        let block_num: u32 = MAX_BLOCK_HEADERS_PER_RESPONSE * 2 + 3;
        let network = Network::Regtest;

        let blobs: Vec<Vec<u8>> =
            helper_initialize_and_get_header_blobs(stability_threshold, block_num, network);

        check_response(&blobs, 0, None, block_num);
        check_response(&blobs, MAX_BLOCK_HEADERS_PER_RESPONSE / 2, None, block_num);
        check_response(&blobs, MAX_BLOCK_HEADERS_PER_RESPONSE, None, block_num);

        check_response(
            &blobs,
            0,
            Some(MAX_BLOCK_HEADERS_PER_RESPONSE + 1),
            block_num,
        );
        check_response(
            &blobs,
            MAX_BLOCK_HEADERS_PER_RESPONSE / 2,
            Some(3 * MAX_BLOCK_HEADERS_PER_RESPONSE / 2 + 1),
            block_num,
        );
        check_response(
            &blobs,
            MAX_BLOCK_HEADERS_PER_RESPONSE,
            Some(2 * MAX_BLOCK_HEADERS_PER_RESPONSE + 1),
            block_num,
        );

        check_response(
            &blobs,
            0,
            Some(2 * MAX_BLOCK_HEADERS_PER_RESPONSE + 1),
            block_num,
        );
    }

    #[test]
    fn get_block_headers_proptest() {
        let stability_threshold = 3;
        let block_num = 200;
        let blobs: Vec<Vec<u8>> = helper_initialize_and_get_header_blobs(
            stability_threshold,
            block_num,
            Network::Regtest,
        );

        proptest!(|(
            start_height in 0..=block_num - 1,
            length in 1..=block_num)|{
                let end_height = if start_height + length - 1 < block_num {
                    Some(start_height + length - 1)
                } else {
                    None
                };
                check_response(&blobs, start_height, end_height, block_num);
            }
        );
    }
}
