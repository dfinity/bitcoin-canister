use ic_cdk::call::CallResult;
use ic_cdk::management_canister::{HttpRequestArgs, HttpRequestResult};

/// Make a HTTP request to a given URL and return HTTP response, possibly after a transformation.
pub async fn http_request(arg: HttpRequestArgs, cycles: u128) -> CallResult<(HttpRequestResult,)> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Mocking cycles is not implemented at the moment.
        let _ = cycles;
        crate::mock::http_request(arg).await
    }

    #[cfg(target_arch = "wasm32")]
    {
        Ok(ic_cdk::call::Call::unbounded_wait(
            candid::Principal::management_canister(),
            "http_request",
        )
        .with_args(&(arg,))
        .with_cycles(cycles)
        .await?
        .candid()?)
    }
}
