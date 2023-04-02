use ic_cdk::api::management_canister::http_request::{HttpHeader, HttpResponse};

/// Creates a new `HttpResponseBuilder` to construct an HTTP response.
pub fn create_response() -> HttpResponseBuilder {
    HttpResponseBuilder::new()
}

/// Represents a builder for an HTTP response.
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
            status: candid::Nat::from(200),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn status(mut self, status: u64) -> Self {
        self.status = candid::Nat::from(status);
        self
    }

    pub fn header(mut self, header: HttpHeader) -> Self {
        self.headers.push(header);
        self
    }

    pub fn body(mut self, body: &str) -> Self {
        self.body = body.as_bytes().to_vec();
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
