use crate::print;
use candid::Principal;
use ic_cdk::api::call::RejectionCode;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,
    TransformContext, TransformFunc,
};

/// The result of a Call.
///
/// Errors on the IC have two components; a Code and a message associated with it.
pub type CallResult<R> = Result<R, (RejectionCode, String)>;

/// Calls the http_request function.
pub async fn http_request(arg: CanisterHttpRequestArgument) -> CallResult<(HttpResponse,)> {
    ic_cdk::api::management_canister::http_request::http_request(arg).await
}

// TODO: remove after switching to ic-http-mock.
pub type TransformFn = fn(TransformArgs) -> HttpResponse;

// TODO: update the content after switching to ic-http-mock.
/// A mock for the TransformContext.
#[cfg(not(target_arch = "wasm32"))]
pub fn build_transform_context(func: TransformFn, context: Vec<u8>) -> TransformContext {
    fn get_function_name<F>(_: F) -> &'static str {
        let full_name = std::any::type_name::<F>();
        match full_name.rfind(':') {
            Some(index) => &full_name[index + 1..],
            None => full_name,
        }
    }

    let function_name = get_function_name(func).to_string();
    TransformContext {
        function: TransformFunc(candid::Func {
            principal: Principal::anonymous(),
            method: function_name,
        }),
        context,
    }
}

/// Creates a TransformContext.
#[cfg(target_arch = "wasm32")]
pub fn build_transform_context<T>(func: T, context: Vec<u8>) -> TransformContext
where
    T: Fn(TransformArgs) -> HttpResponse,
{
    TransformContext::new(func, context)
}

/// Performs a http_request and returns the body of the response.
pub async fn fetch_body(request: CanisterHttpRequestArgument) -> Result<String, String> {
    match http_request(request).await {
        Ok((response,)) => {
            if response.status == 200 {
                let body = String::from_utf8(response.body)
                    .expect("Transformed response is not UTF-8 encoded.");
                Ok(body)
            } else {
                let message = format!(
                    "The http_request resulted into error with status: {:?}",
                    response.status
                );
                print(&message);
                Err(message)
            }
        }
        Err((r, m)) => {
            let message =
                format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");
            print(&message);
            Err(message)
        }
    }
}

/// Creates a CanisterHttpRequestArgument.
pub fn create_request(
    host: String,
    url: String,
    max_response_bytes: Option<u64>,
    transform: Option<TransformContext>,
) -> CanisterHttpRequestArgument {
    CanisterHttpRequestArgument {
        url,
        method: HttpMethod::GET,
        body: None,
        max_response_bytes,
        transform,
        headers: vec![
            HttpHeader {
                name: "Host".to_string(),
                value: format!("{host}:443"),
            },
            HttpHeader {
                name: "User-Agent".to_string(),
                value: "bitcoin_watchdog_canister".to_string(),
            },
        ],
    }
}

/// Applies a function to the body of the response assuming it contains JSON.
pub fn apply_to_body_json(
    raw: TransformArgs,
    function: fn(serde_json::Value) -> serde_json::Value,
) -> HttpResponse {
    let mut response = HttpResponse {
        status: raw.response.status.clone(),
        ..Default::default()
    };
    if response.status == 200 {
        let body =
            String::from_utf8(raw.response.body).expect("Raw response is not UTF-8 encoded.");
        let original = serde_json::from_str(body.as_str())
            .unwrap_or_else(|_| panic!("Can't parse JSON from a raw response, body={}", body));
        let modified = function(original);
        response.body = modified.to_string().as_bytes().to_vec();
    } else {
        print(&format!("Received an error: err = {:?}", raw));
    }
    response
}
