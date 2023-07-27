use ic_cdk::api::call::RejectionCode;
use ic_cdk::api::management_canister::http_request::{CanisterHttpRequestArgument, HttpResponse};

/// The result of a Call.
///
/// Errors on the IC have two components; a Code and a message associated with it.
pub type CallResult<R> = Result<R, (RejectionCode, String)>;

/// Make a HTTP request to a given URL and return HTTP response, possibly after a transformation.
pub async fn http_request(
    arg: CanisterHttpRequestArgument,
    cycles: u128,
) -> CallResult<(HttpResponse,)> {
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
