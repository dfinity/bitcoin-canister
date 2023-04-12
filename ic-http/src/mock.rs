use ic_cdk::api::call::RejectionCode;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpResponse, TransformArgs,
};
use std::time::Duration;

/// Represents a mock HTTP request and its corresponding response.
#[derive(Clone)]
pub(crate) struct Mock {
    pub(crate) request: CanisterHttpRequestArgument,
    response: HttpResponse,
    delay: Duration,
    times_called: u64,
}

/// Adds a mock for a given HTTP request and response. The mock will be returned by
/// subsequent calls to `http_request` that match the same request. The response will be
/// returned immediately, without any delay.
pub fn mock(request: CanisterHttpRequestArgument, response: HttpResponse) {
    mock_with_delay(request, response, Duration::from_secs(0));
}

/// Adds a mock for a given HTTP request and response. The mock will be returned by
/// subsequent calls to `http_request` that match the same request. The response will be
/// returned after a delay specified by the `delay` argument.
pub fn mock_with_delay(
    request: CanisterHttpRequestArgument,
    response: HttpResponse,
    delay: Duration,
) {
    crate::storage::mock_insert(Mock {
        request,
        response,
        delay,
        times_called: 0,
    });
}

/// Handles incoming HTTP requests by retrieving a mock response based
/// on the request, possibly delaying the response, transforming the response if necessary,
/// and returning it. If there is no mock found, it returns an error.
pub(crate) async fn http_request(
    request: CanisterHttpRequestArgument,
) -> Result<(HttpResponse,), (RejectionCode, String)> {
    let mut mock = crate::storage::mock_get(&request)
        .ok_or((RejectionCode::CanisterReject, "No mock found".to_string()))?;
    mock.times_called += 1;
    crate::storage::mock_insert(mock.clone());

    // Delay the response if necessary.
    if mock.delay > Duration::from_secs(0) {
        // TODO: Use a non-blocking sleep when the CDK supports it.
        #[cfg(not(target_arch = "wasm32"))]
        tokio::time::sleep(mock.delay).await;
    }

    // Check if the response body exceeds the maximum allowed size.
    if let Some(max_response_bytes) = mock.request.max_response_bytes {
        if mock.response.body.len() as u64 > max_response_bytes {
            return Err((
                RejectionCode::SysFatal,
                format!(
                    "Value of 'Content-length' header exceeds http body size limit, {} > {}.",
                    mock.response.body.len(),
                    max_response_bytes
                ),
            ));
        }
    }

    // Apply the transform function if one is specified.
    let transformed_response = request
        .transform
        .and_then(|t| {
            crate::storage::transform_function_call(
                t.function.0.method,
                TransformArgs {
                    response: mock.response.clone(),
                    context: vec![],
                },
            )
        })
        .unwrap_or_else(|| mock.response.clone());

    Ok((transformed_response,))
}

/// Returns the number of times the given request has been called.
/// Returns 0 if no mock has been found for the request.
pub fn times_called(request: CanisterHttpRequestArgument) -> u64 {
    crate::storage::mock_get(&request)
        .map(|mock| mock.times_called)
        .unwrap_or(0)
}

/// Returns a sorted list of registered transform function names.
pub fn registered_transform_function_names() -> Vec<String> {
    crate::storage::transform_function_names()
}

/// Create a hash from a `CanisterHttpRequestArgument`, which includes its URL,
/// method, headers, body, and optionally, its transform function name.
/// This is because `CanisterHttpRequestArgument` does not have `Hash` implemented.
pub(crate) fn hash(request: &CanisterHttpRequestArgument) -> String {
    let mut hash = String::new();

    hash.push_str(&request.url);
    hash.push_str(&format!("{:?}", request.max_response_bytes));
    hash.push_str(&format!("{:?}", request.method));
    for header in request.headers.iter() {
        hash.push_str(&header.name);
        hash.push_str(&header.value);
    }
    let body = String::from_utf8(request.body.as_ref().unwrap_or(&vec![]).clone())
        .expect("Raw response is not UTF-8 encoded.");
    hash.push_str(&body);
    let function_name = request
        .transform
        .as_ref()
        .map(|transform| transform.function.0.method.clone());
    if let Some(name) = function_name {
        hash.push_str(&name);
    }

    hash
}
