use crate::print;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpHeader, HttpResponse, TransformArgs,
};

pub type TransformFn = fn(TransformArgs) -> HttpResponse;

pub struct HttpRequestConfig {
    url: String,
    transform_endpoint: Option<TransformFn>,
    transform_implementation: TransformFn,
}

impl HttpRequestConfig {
    pub fn new(
        url: &str,
        transform_endpoint: Option<TransformFn>,
        transform_implementation: TransformFn,
    ) -> Self {
        Self {
            url: String::from(url),
            transform_endpoint,
            transform_implementation,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn transform(&self, raw: TransformArgs) -> HttpResponse {
        (self.transform_implementation)(raw)
    }

    pub fn create_request(&self) -> CanisterHttpRequestArgument {
        create_request(&self.url, self.transform_endpoint.map(|func| func))
    }
}

pub async fn fetch_body(request: CanisterHttpRequestArgument) -> String {
    let result = ic_http::http_request(request.clone()).await;

    match result {
        Ok((response,)) => {
            let body = String::from_utf8(response.body).unwrap();
            print(&format!("Response: {:?}", body));
            body
        }
        Err((code, msg)) => {
            print(&format!("Error: {:?} {:?}", code, msg));
            String::from("")
        }
    }
}

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
