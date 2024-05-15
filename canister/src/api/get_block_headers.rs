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

/// Retrieves the block headers of the given starting height.
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
