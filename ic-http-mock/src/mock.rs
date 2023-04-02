use ic_cdk::api::call::RejectionCode;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpResponse, TransformArgs,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

// A thread-local hashmap that stores mocks.
thread_local! {
    static MOCKS: RefCell<HashMap<String, Mock>> = RefCell::default();
}

/// Inserts the provided mock into a thread-local hashmap.
fn insert(mock: Mock) {
    MOCKS.with(|cell| cell.borrow_mut().insert(hash(&mock.request), mock));
}

/// Returns a cloned mock from the thread-local hashmap that corresponds to the provided request.
fn get(request: &CanisterHttpRequestArgument) -> Option<Mock> {
    MOCKS.with(|cell| cell.borrow().get(&hash(request)).cloned())
}

/// Represents a mock HTTP request and its corresponding response.
#[derive(Clone)]
struct Mock {
    request: CanisterHttpRequestArgument,
    response: HttpResponse,
    delay: Duration,
    times_called: u64,
}

/// Adds a mock for a given HTTP request and response. The mock will be returned by
/// subsequent calls to `http_request` that match the same request. The response will be
/// returned immediately, without any delay.
pub fn mock(request: &CanisterHttpRequestArgument, response: &HttpResponse) {
    mock_with_delay(request, response, Duration::from_secs(0));
}

/// Adds a mock for a given HTTP request and response. The mock will be returned by
/// subsequent calls to `http_request` that match the same request. The response will be
/// returned after a delay specified by the `delay` argument.
pub fn mock_with_delay(
    request: &CanisterHttpRequestArgument,
    response: &HttpResponse,
    delay: Duration,
) {
    insert(Mock {
        request: request.clone(),
        response: response.clone(),
        delay,
        times_called: 0,
    });
}

/// Handles incoming HTTP requests by retrieving a mock response based
/// on the request, possibly delaying the response, transforming the response if necessary,
/// and returning it. If there is no mock found, it returns an error.
pub async fn http_request(
    request: &CanisterHttpRequestArgument,
) -> Result<(HttpResponse,), (RejectionCode, String)> {
    let mut mock =
        get(request).ok_or((RejectionCode::CanisterReject, "No mock found".to_string()))?;
    mock.times_called += 1;
    insert(mock.clone());

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
    let transform = match request.transform {
        Some(ref t) => crate::transform::registry_get(t.function.0.method.clone()),
        None => None,
    };
    let transformed_response = match transform {
        Some(function) => function(TransformArgs {
            response: mock.response.clone(),
            context: vec![],
        }),
        None => mock.response.clone(),
    };

    Ok((transformed_response,))
}

/// Returns the number of times the given request has been called.
/// Returns 0 if no mock has been found for the request.
pub fn times_called(request: &CanisterHttpRequestArgument) -> u64 {
    get(request).map(|mock| mock.times_called).unwrap_or(0)
}

/// Create a hash from a CanisterHttpRequestArgument, which includes its URL,
/// method, headers, body, and optionally, its transform function name.
fn hash(request: &CanisterHttpRequestArgument) -> String {
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
