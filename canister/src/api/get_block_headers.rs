use ic_btc_interface::{GetBlockHeadersError, GetBlockHeadersRequest, GetBlockHeadersResponse};

// Various profiling stats for tracking the performance of `get_block_headers`.
#[derive(Default, Debug)]
struct Stats {
    // The total number of instructions used to process the request.
    _ins_total: u64,

    // The number of instructions used to build the block headers vec.
    _ins_build_block_headers_vec: u64,
}

/// Retrieves the block headers of the given starting height.
pub fn get_block_headers(
    _request: GetBlockHeadersRequest,
) -> Result<GetBlockHeadersResponse, GetBlockHeadersError> {
    Err(GetBlockHeadersError::StartHeightLagrerThanEndHeight)
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
        assert_eq!(
            get_block_headers(GetBlockHeadersRequest {
                start_height: 3,
                end_height: Some(2),
            }),
            Err(GetBlockHeadersError::StartHeightLagrerThanEndHeight)
        );
    }
}
