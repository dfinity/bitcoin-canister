use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpHeader, HttpMethod, TransformContext,
};

/// Creates a new `HttpRequestBuilder` to construct an HTTP request.
pub fn create_request() -> HttpRequestBuilder {
    HttpRequestBuilder::new()
}

/// Represents a builder for an HTTP request.
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
            url: String::new(),
            max_response_bytes: None,
            method: HttpMethod::GET,
            headers: Vec::new(),
            body: None,
            transform: None,
        }
    }

    pub fn get(mut self, url: &str) -> Self {
        self.url = url.to_string();
        self.method = HttpMethod::GET;
        self
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = url.to_string();
        self
    }

    pub fn max_response_bytes(mut self, max_response_bytes: u64) -> Self {
        self.max_response_bytes = Some(max_response_bytes);
        self
    }

    pub fn method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    pub fn header(mut self, header: HttpHeader) -> Self {
        self.headers.push(header);
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn transform(mut self, transform: TransformContext) -> Self {
        self.transform = Some(transform);
        self
    }

    pub fn build(self) -> CanisterHttpRequestArgument {
        CanisterHttpRequestArgument {
            url: self.url,
            max_response_bytes: self.max_response_bytes,
            method: self.method,
            headers: self.headers,
            body: self.body,
            transform: self.transform,
        }
    }
}

impl Default for HttpRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}
