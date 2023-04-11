use crate::print;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpHeader, HttpResponse, TransformArgs,
};

pub type TransformFn = fn(TransformArgs) -> HttpResponse;

pub struct HttpRequestConfig {
    url: String,
    request: CanisterHttpRequestArgument,
    transform_implementation: TransformFn,
}

impl HttpRequestConfig {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(
        url: &str,
        transform_endpoint: Option<TransformFn>,
        transform_implementation: TransformFn,
    ) -> Self {
        Self {
            url: String::from(url),
            request: create_request(url, transform_endpoint),
            transform_implementation,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new<T>(
        url: &str,
        transform_endpoint: Option<T>,
        transform_implementation: TransformFn,
    ) -> Self
    where
        T: Fn(TransformArgs) -> HttpResponse,
    {
        Self {
            url: String::from(url),
            request: create_request(url, transform_endpoint),
            transform_implementation,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn transform(&self, raw: TransformArgs) -> HttpResponse {
        (self.transform_implementation)(raw)
    }

    pub fn request(&self) -> CanisterHttpRequestArgument {
        self.request.clone()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn create_request(
    url: &str,
    transform_func: Option<TransformFn>,
) -> CanisterHttpRequestArgument {
    let builder = ic_http::create_request().get(url).header(HttpHeader {
        name: "User-Agent".to_string(),
        value: "bitcoin_watchdog_canister".to_string(),
    });
    let builder = if let Some(func) = transform_func {
        builder.transform_func(func, vec![])
    } else {
        builder
    };
    builder.build()
}

#[cfg(target_arch = "wasm32")]
pub fn create_request<T>(url: &str, transform_func: Option<T>) -> CanisterHttpRequestArgument
where
    T: Fn(TransformArgs) -> HttpResponse,
{
    let builder = ic_http::create_request().get(url).header(HttpHeader {
        name: "User-Agent".to_string(),
        value: "bitcoin_watchdog_canister".to_string(),
    });
    let builder = if let Some(func) = transform_func {
        builder.transform_func(func, vec![])
    } else {
        builder
    };
    builder.build()
}
