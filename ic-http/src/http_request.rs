use ic_cdk::api::call::RejectionCode;
use ic_cdk::api::management_canister::http_request::{CanisterHttpRequestArgument, HttpResponse};

/// The result of a Call.
///
/// Errors on the IC have two components; a Code and a message associated with it.
pub type CallResult<R> = Result<R, (RejectionCode, String)>;

/// Make a mocked HTTP request to a given URL and return mocked HTTP response, possibly after a transformation.
#[cfg(not(target_arch = "wasm32"))]
pub async fn http_request(arg: CanisterHttpRequestArgument) -> CallResult<(HttpResponse,)> {
    crate::mock::http_request(arg).await
}

/// Make an HTTP request to a given URL and return the HTTP response, possibly after a transformation.
#[cfg(target_arch = "wasm32")]
pub async fn http_request(arg: CanisterHttpRequestArgument) -> CallResult<(HttpResponse,)> {
    ic_cdk::api::management_canister::http_request::http_request(arg).await
}
