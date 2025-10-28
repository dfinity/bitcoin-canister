use ic_cdk::call::RejectCode;
use ic_cdk::management_canister::{HttpRequestArgs, HttpRequestResult};

/// The result of a Call.
///
/// Errors on the IC have two components; a Code and a message associated with it.
pub type CallResult<R> = Result<R, (RejectCode, String)>;

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
        ic_cdk::api::call::call_with_payment128(
            candid::Principal::management_canister(),
            "http_request",
            (arg,),
            cycles,
        )
        .await
    }
}
