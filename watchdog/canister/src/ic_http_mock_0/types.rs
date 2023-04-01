use crate::ic_http_mock::transform::TransformContextBuilder;
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,
    TransformContext,
};

pub type TransformFn = fn(TransformArgs) -> HttpResponse;

pub struct HttpRequestBuilder {
    /// The requested URL.
    pub url: String,
    /// The maximal size of the response in bytes. If None, 2MiB will be the limit.
    pub max_response_bytes: Option<u64>,
    /// The method of HTTP request.
    pub method: HttpMethod,
    /// List of HTTP request headers and their corresponding values.
    pub headers: Vec<HttpHeader>,
    /// Optionally provide request body.
    pub body: Option<Vec<u8>>,
    /// Name of the transform function which is `func (transform_args) -> (http_response) query`.
    pub transform: Option<TransformContext>,
}

impl HttpRequestBuilder {
    pub fn new() -> Self {
        Self {
            url: "".to_string(),
            method: HttpMethod::GET,
            body: None,
            max_response_bytes: None,
            transform: None,
            headers: vec![],
        }
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = url.to_string();
        self
    }

    pub fn method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    pub fn transform(mut self, func: TransformFn, context: Vec<u8>) -> Self {
        let transform = TransformContextBuilder::new()
            .func(func)
            .context(context)
            .build();
        self.transform = Some(transform);
        self
    }

    pub fn max_response_bytes(mut self, max_response_bytes: u64) -> Self {
        self.max_response_bytes = Some(max_response_bytes);
        self
    }

    pub fn build(self) -> CanisterHttpRequestArgument {
        CanisterHttpRequestArgument {
            url: self.url,
            method: self.method,
            body: self.body,
            max_response_bytes: self.max_response_bytes,
            transform: self.transform,
            headers: self.headers,
        }
    }
}

impl Default for HttpRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HttpResponseBuilder {
    /// The response status (e.g., 200, 404).
    pub status: candid::Nat,
    /// List of HTTP response headers and their corresponding values.
    pub headers: Vec<HttpHeader>,
    /// The response’s body.
    pub body: Vec<u8>,
}

impl HttpResponseBuilder {
    pub fn new() -> Self {
        Self {
            status: candid::Nat::from(-1),
            headers: vec![],
            body: vec![],
        }
    }

    pub fn status(mut self, status: i32) -> Self {
        self.status = candid::Nat::from(status);
        self
    }

    pub fn headers(mut self, headers: &[HttpHeader]) -> Self {
        self.headers = headers.to_vec();
        self
    }

    pub fn body(mut self, text: &str) -> Self {
        self.body = text.to_string().as_bytes().to_vec();
        self
    }

    pub fn build(self) -> HttpResponse {
        HttpResponse {
            status: self.status,
            headers: self.headers,
            body: self.body,
        }
    }
}

impl Default for HttpResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}
