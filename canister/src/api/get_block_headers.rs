use ic_btc_interface::{GetBlockHeadersError, GetBlockHeadersRequest, GetBlockHeadersResponse};

use crate::{charge_cycles, runtime::print, verify_has_enough_cycles, with_state, with_state_mut};

// Various profiling stats for tracking the performance of `get_block_headers`.
#[derive(Default, Debug)]
struct Stats {
    // The total number of instructions used to process the request.
    ins_total: u64,

    // The number of instructions used to build the block headers vec.
    ins_build_block_headers_vec: u64,
}

fn verify_get_block_headers_request(
    request: &GetBlockHeadersRequest,
) -> Result<(), GetBlockHeadersError> {
    if let Some(end_height) = request.end_height {
        if end_height < request.start_height {
            return Err(GetBlockHeadersError::StartHeightLagerThanEndHeight {
                start_height: request.start_height,
                end_height,
            });
        }
    }
    Ok(())
}

fn get_block_headers_internal(
    request: &GetBlockHeadersRequest,
) -> Result<(GetBlockHeadersResponse, Stats), GetBlockHeadersError> {
    unimplemented!("get_block_headers_internal");
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

    verify_get_block_headers_request(&request)?;

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
    use ic_btc_interface::{Config, GetBlockHeadersError, GetBlockHeadersRequest, Network};

    use crate::api::get_block_headers;

    #[test]
    fn get_block_headers_malformed_heights() {
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

        assert_eq!(
            err,
            GetBlockHeadersError::StartHeightLagerThanEndHeight {
                start_height,
                end_height,
            }
        );
    }
}
