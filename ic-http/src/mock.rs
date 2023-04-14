use ic_cdk::api::call::RejectionCode;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpResponse, TransformArgs,
};
use std::time::Duration;

/// Represents a mock HTTP request and its corresponding response.
#[derive(Clone)]
pub(crate) struct Mock {
    pub(crate) request: CanisterHttpRequestArgument,
    result: Option<Result<HttpResponse, (RejectionCode, String)>>,
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
        result: Some(Ok(response)),
        delay,
        times_called: 0,
    });
}

/// Adds a mock error for a given HTTP request and error. The mock will be returned by
/// subsequent calls to `http_request` that match the same request. The error will be
/// returned immediately, without any delay.
pub fn mock_error(request: CanisterHttpRequestArgument, error: (RejectionCode, String)) {
    mock_error_with_delay(request, error, Duration::from_secs(0));
}

/// Adds a mock error for a given HTTP request and error. The mock will be returned by
/// subsequent calls to `http_request` that match the same request. The error will be
/// returned after a delay specified by the `delay` argument.
pub fn mock_error_with_delay(
    request: CanisterHttpRequestArgument,
    error: (RejectionCode, String),
    delay: Duration,
) {
    crate::storage::mock_insert(Mock {
        request,
        result: Some(Err(error)),
        delay,
        times_called: 0,
    });
}

/// Calls the transform function if one is specified in the request.
pub fn call_transform_function(
    request: CanisterHttpRequestArgument,
    arg: TransformArgs,
) -> Option<HttpResponse> {
    request
        .transform
        .and_then(|t| crate::storage::transform_function_call(t.function.0.method, arg))
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

    let mock_response = match mock.result {
        None => panic!("Mock response is missing"),
        // Return the error if one is specified.
        Some(Err(error)) => return Err(error),
        Some(Ok(response)) => response,
    };

    // Check if the response body exceeds the maximum allowed size.
    if let Some(max_response_bytes) = mock.request.max_response_bytes {
        if mock_response.body.len() as u64 > max_response_bytes {
            return Err((
                RejectionCode::SysFatal,
                format!(
                    "Value of 'Content-length' header exceeds http body size limit, {} > {}.",
                    mock_response.body.len(),
                    max_response_bytes
                ),
            ));
        }
    }

    // Apply the transform function if one is specified.
    let transformed_response = call_transform_function(
        mock.request,
        TransformArgs {
            response: mock_response.clone(),
            context: vec![],
        },
    )
    .unwrap_or(mock_response);

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
