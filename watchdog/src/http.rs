use crate::config::SubnetType;
use ic_cdk::call::CallResult;
#[cfg(target_arch = "wasm32")]
use ic_cdk::management_canister::cost_http_request;
use ic_cdk::management_canister::{HttpHeader, HttpRequestArgs, HttpRequestResult, TransformArgs};
use serde_json::json;

pub type TransformFn = fn(TransformArgs) -> HttpRequestResult;

/// Stores the configuration of the HTTP request, which includes:
/// - URL
/// - HTTP request
/// - Transform function inner implementation to be called inside the canister's endpoint
pub struct HttpRequestConfig {
    request: HttpRequestArgs,
    transform_implementation: TransformFn,
}

pub struct TransformFnWrapper<T> {
    pub name: &'static str,
    pub func: T,
}

impl HttpRequestConfig {
    pub fn new<T>(
        url: &str,
        transform_endpoint: Option<TransformFnWrapper<T>>,
        transform_implementation: TransformFn,
    ) -> Self
    where
        T: Fn(TransformArgs) -> HttpRequestResult + 'static,
    {
        Self {
            request: create_request(url, transform_endpoint),
            transform_implementation,
        }
    }

    /// Executes the transform function.
    pub fn transform(&self, raw: TransformArgs) -> HttpRequestResult {
        (self.transform_implementation)(raw)
    }

    /// Returns the HTTP request arguments.
    #[cfg(test)]
    pub fn request(&self) -> HttpRequestArgs {
        self.request.clone()
    }

    /// Returns the request URL.
    #[cfg(test)]
    pub fn url(&self) -> String {
        self.request.url.clone()
    }

    /// Calculates the number of cycles to attach to the HTTP request based on subnet type.
    fn calculate_cycles(&self, subnet_type: SubnetType) -> u128 {
        match subnet_type {
            // Send zero cycles with the request to avoid the canister
            // to run out of cycles when deployed on a system subnet.
            SubnetType::System => 0,
            SubnetType::Application => {
                #[cfg(target_arch = "wasm32")]
                {
                    cost_http_request(&self.request)
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    0
                }
            }
        }
    }

    /// Sends the HTTP request.
    pub async fn send_request(&self) -> CallResult<HttpRequestResult> {
        let canister = crate::storage::get_canister();
        let cycles = self.calculate_cycles(canister.subnet_type());
        ic_http::http_request(self.request.clone(), cycles).await
    }

    /// Sends the HTTP request and parses the response as JSON.
    pub async fn send_request_json(&self) -> serde_json::Value {
        match self.send_request().await {
            Ok(response) if response.status == 200u8 => self.parse_json_response(response),
            Ok(response) => {
                crate::print(&format!(
                    "HTTP request returned non-200 status: {}. Response body: {}",
                    response.status,
                    String::from_utf8_lossy(&response.body)
                ));
                json!({})
            }
            Err(error) => {
                crate::print(&format!("HTTP request failed: {:?}", error));
                json!({})
            }
        }
    }

    /// Parses the given HTTP response into a JSON value.
    fn parse_json_response(&self, response: HttpRequestResult) -> serde_json::Value {
        serde_json::from_slice(&response.body).unwrap_or_else(|error| {
            crate::print(&format!(
                "Failed to parse JSON, error: {error:?}, body: {}",
                String::from_utf8_lossy(&response.body)
            ));
            json!({})
        })
    }
}

fn create_request<T>(url: &str, transform_func: Option<TransformFnWrapper<T>>) -> HttpRequestArgs
where
    T: Fn(TransformArgs) -> HttpRequestResult + 'static,
{
    let builder = ic_http::create_request().get(url).header(HttpHeader {
        name: "User-Agent".to_string(),
        value: "watchdog_canister".to_string(),
    });
    let builder = if let Some(func) = transform_func {
        builder.transform_func(func.name, func.func, vec![])
    } else {
        builder
    };
    builder.build()
}
