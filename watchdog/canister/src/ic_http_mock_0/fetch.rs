use ic_cdk::api::call::RejectionCode;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpResponse, TransformArgs,
};

/// The result of a Call.
///
/// Errors on the IC have two components; a Code and a message associated with it.
pub type CallResult<R> = Result<R, (RejectionCode, String)>;

/// Performs a http call with mocked data.
/// Expected to be a wrapper around `ic_cdk::api::management_canister::http_request::http_request(...)`.
pub async fn fetch(arg: CanisterHttpRequestArgument) -> CallResult<(HttpResponse,)> {
    match crate::ic_http_mock::mock::get(&arg) {
        Some(entry) => {
            let mut response = entry.response;

            if let Some(duration) = entry.delay {
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(duration).await;
            }

            if let Some(max_response_bytes) = arg.max_response_bytes {
                if response.body.len() as u64 > max_response_bytes {
                    return Err((
                        RejectionCode::SysFatal,
                        format!("Value of 'Content-length' header exceeds http body size limit, {} > {}.", response.body.len(), max_response_bytes),
                    ));
                }
            }
            let method = arg.transform.map(|context| context.function.0.method);
            let transform = method.and_then(crate::ic_http_mock::transform::get);
            if let Some(function) = transform {
                response = function(TransformArgs {
                    response,
                    context: vec![],
                });
            }
            Ok((response,))
        }
        None => Err((RejectionCode::Unknown, "No response found".to_string())),
    }
}
