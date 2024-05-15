use ic_btc_interface::{GetBlockHeadersError, GetBlockHeadersRequest, GetBlockHeadersResponse};

// Various profiling stats for tracking the performance of `get_block_headers`.
#[derive(Default, Debug)]
struct Stats {
    // The total number of instructions used to process the request.
    _ins_total: u64,

    // The number of instructions used to build the block headers vec.
    _ins_build_block_headers_vec: u64,
}

fn verify_get_block_headers_request(
    request: GetBlockHeadersRequest,
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
    verify_get_block_headers_request(request)?;
    unimplemented!("get_block_headers is not implemented")
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
