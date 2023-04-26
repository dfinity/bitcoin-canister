use ic_cdk::api::call::RejectionCode;
use ic_cdk::api::management_canister::http_request::{CanisterHttpRequestArgument, HttpResponse};

/// The result of a Call.
///
/// Errors on the IC have two components; a Code and a message associated with it.
pub type CallResult<R> = Result<R, (RejectionCode, String)>;

/// Make a HTTP request to a given URL and return HTTP response, possibly after a transformation.
#[cfg(not(target_arch = "wasm32"))]
pub async fn http_request(arg: CanisterHttpRequestArgument) -> CallResult<(HttpResponse,)> {
    crate::mock::http_request(arg).await
}

/// Make an HTTP request to a given URL and return the HTTP response, possibly after a transformation.
#[cfg(target_arch = "wasm32")]
pub async fn http_request(arg: CanisterHttpRequestArgument) -> CallResult<(HttpResponse,)> {
    ic_cdk::api::management_canister::http_request::http_request(arg).await
}

/// Make a HTTP request to a given URL and return HTTP response, possibly after a transformation.
#[cfg(not(target_arch = "wasm32"))]
pub async fn http_request_with_cycles(
    arg: CanisterHttpRequestArgument,
    _cycles: u128,
) -> CallResult<(HttpResponse,)> {
    crate::mock::http_request(arg).await
}

/// Make an HTTP request to a given URL and return the HTTP response, possibly after a transformation.
#[cfg(target_arch = "wasm32")]
pub async fn http_request_with_cycles(
    arg: CanisterHttpRequestArgument,
    cycles: u128,
) -> CallResult<(HttpResponse,)> {
    ic_cdk::api::management_canister::http_request::http_request_with_cycles(arg, cycles).await
}
