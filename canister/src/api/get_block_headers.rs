use ic_btc_interface::{GetBlockHeadersError, GetBlockHeadersRequest, GetBlockHeadersResponse};

use crate::{
    charge_cycles,
    runtime::{performance_counter, print},
    verify_has_enough_cycles, with_state, with_state_mut,
};

// Various profiling stats for tracking the performance of `get_block_headers`.
#[derive(Default, Debug)]
struct Stats {
    // The total number of instructions used to process the request.
    ins_total: u64,

    // The number of instructions used to build the block headers vec.
    ins_build_block_headers_vec: u64,
}

fn verify_requested_height_range_and_return_effective_range(
    request: &GetBlockHeadersRequest,
) -> Result<(u32, u32), GetBlockHeadersError> {
    let chain_height =
        if let Some(chain_height) = with_state(|s| s.stable_block_headers.chain_height()) {
            chain_height
        } else {
            return Err(GetBlockHeadersError::ChainIsEmpty);
        };

    if request.start_height > chain_height {
        return Err(GetBlockHeadersError::StartHeightDoesNotExist {
            requested: request.start_height,
            chain_height,
        });
    }

    if let Some(end_height) = request.end_height {
        if end_height < request.start_height {
            return Err(GetBlockHeadersError::StartHeightLagerThanEndHeight {
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
        // If end_height is provided then it should be the
        // end boundary of effective height range.
        Ok((request.start_height, end_height))
    } else {
        // If end_height is not provided than the end of
        // effective range should be the last block of chain.
        Ok((request.start_height, chain_height))
    }
}

fn get_block_headers_internal(
    request: &GetBlockHeadersRequest,
) -> Result<(GetBlockHeadersResponse, Stats), GetBlockHeadersError> {
    let (start_height, end_height) =
        verify_requested_height_range_and_return_effective_range(request)?;

    let mut stats: Stats = Stats::default();

    // Build block headers vec.
    let ins_start = performance_counter();

    let vec_headers = with_state(|s| {
        let block_heights = &s.stable_block_headers.block_heights;
        let block_headers = &s.stable_block_headers.block_headers;
        block_heights
            .range(start_height..end_height)
            .map(|(_, block_hash)| block_headers.get(&block_hash).unwrap().into())
            .collect()
    });

    stats.ins_build_block_headers_vec = performance_counter() - ins_start;
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
    verify_has_enough_cycles(with_state(|s| s.fees.get_block_headers_maximum));
    // Charge the base fee.
    charge_cycles(with_state(|s| s.fees.get_block_headers_base));

    let (res, stats) = get_block_headers_internal(&request)?;

    // Observe metrics
    with_state_mut(|s| {
        s.metrics.get_block_headers_total.observe(stats.ins_total);

        s.metrics
            .get_block_headers_build_block_headers_vec
            .observe(stats.ins_build_block_headers_vec);
    });

    // Charge the fee based on the number of the instructions.
    with_state(|s| {
        let fee = std::cmp::min(
            (stats.ins_total / 10) as u128 * s.fees.get_block_headers_cycles_per_ten_instructions,
            s.fees.get_block_headers_maximum - s.fees.get_block_headers_base,
        );

        charge_cycles(fee);
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
        state::{ingest_stable_blocks_into_utxoset, insert_block},
        test_utils::BlockBuilder,
        with_state_mut,
    };
    use bitcoin::consensus::Encodable;
    use ic_btc_interface::{Config, Network};

    #[test]
    fn get_block_headers_chain_is_empty() {
        crate::init(Config {
            stability_threshold: 1,
            network: Network::Mainnet,
            ..Default::default()
        });

        let start_height = 3;
        let end_height = 2;

        let err = get_block_headers(GetBlockHeadersRequest {
            start_height,
            end_height: Some(end_height),
        })
        .unwrap_err();

        assert_eq!(err, GetBlockHeadersError::ChainIsEmpty);
    }

    fn get_block_headers_helper_one_stable_block() {
        let network = Network::Regtest;
        crate::init(Config {
            stability_threshold: 1,
            network,
            ..Default::default()
        });

        let block1 = BlockBuilder::with_prev_header(genesis_block(network).header()).build();
        let block2 = BlockBuilder::with_prev_header(block1.clone().header()).build();

        // Insert the block.
        with_state_mut(|state| {
            insert_block(state, block1).unwrap();
            insert_block(state, block2).unwrap();
            ingest_stable_blocks_into_utxoset(state);
        });
    }

    #[test]
    fn get_block_headers_malformed_heights() {
        get_block_headers_helper_one_stable_block();

        let start_height = 1;
        let end_height = 0;

        let err = get_block_headers(GetBlockHeadersRequest {
            start_height,
            end_height: Some(end_height),
        })
        .unwrap_err();

        assert_eq!(
            err,
            GetBlockHeadersError::StartHeightLagerThanEndHeight {
                start_height,
                end_height,
            }
        );
    }

    #[test]
    fn start_height_does_not_exist() {
        get_block_headers_helper_one_stable_block();

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
                chain_height: 1
            }
        );
    }

    #[test]
    fn end_height_does_not_exist() {
        get_block_headers_helper_one_stable_block();

        let start_height: u32 = 1;
        let end_height: u32 = 3;

        let err = get_block_headers(GetBlockHeadersRequest {
            start_height,
            end_height: Some(end_height),
        })
        .unwrap_err();

        assert_eq!(
            err,
            GetBlockHeadersError::EndHeightDoesNotExist {
                requested: end_height,
                chain_height: 1
            }
        );
    }

    #[test]
    fn get_block_headers_simple_chain() {
        let network = Network::Regtest;
        crate::init(Config {
            stability_threshold: 1,
            network,
            ..Default::default()
        });

        let block1 = BlockBuilder::with_prev_header(genesis_block(network).header()).build();
        let block2 = BlockBuilder::with_prev_header(block1.clone().header()).build();
        let block3 = BlockBuilder::with_prev_header(block2.clone().header()).build();

        // Insert the blocks.
        // After inserting genesis and block1 should be stable.
        with_state_mut(|state| {
            insert_block(state, block1.clone()).unwrap();
            insert_block(state, block2).unwrap();
            insert_block(state, block3).unwrap();
            ingest_stable_blocks_into_utxoset(state);
        });

        // We request all block headers starting from height 0, until the end of the chain.
        let response: GetBlockHeadersResponse = get_block_headers(GetBlockHeadersRequest {
            start_height: 0,
            end_height: None,
        })
        .unwrap();

        let mut genesis_header_blob = vec![];
        genesis_block(network)
            .header()
            .consensus_encode(&mut genesis_header_blob)
            .unwrap();

        let mut block1_header_blob = vec![];
        block1
            .header()
            .consensus_encode(&mut block1_header_blob)
            .unwrap();

        // The result should contain headers of genesis block and block1.
        assert_eq!(
            response,
            GetBlockHeadersResponse {
                tip_height: 2,
                block_headers: vec![genesis_header_blob, block1_header_blob]
            }
        );
    }
}
