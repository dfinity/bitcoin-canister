use ic_cdk::management_canister::{HttpHeader, HttpRequestArgs, HttpRequestResult, TransformArgs};

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

    /// Returns the HTTP request.
    pub fn request(&self) -> HttpRequestArgs {
        self.request.clone()
    }

    /// Returns the request URL.
    #[cfg(test)]
    pub fn url(&self) -> String {
        self.request.url.clone()
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
