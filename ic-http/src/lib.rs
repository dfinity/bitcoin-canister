//!
//! `ic-http` offers a simplified API to make HTTP outcalls on the Internet Computer with an extra convenience of mocking them in tests.
//!
//! # Getting Started
//!
//! To use `ic-http`, follow these steps:
//! - Add `ic-http` to your `Cargo.toml`
//! - Create a canister
//! - Build a request using the `ic_http::create_request` function
//!   - If necessary, provide a transform function using `ic_http::create_transform_context`
//! - Make an HTTP request using `ic_http::http_request`
//! - Test with mock data by using `ic_http::mock::mock`, etc.
//!
//! ## Canister
//!
//! ```rust
//! # use ic_cdk::api::management_canister::http_request::{
//! #     CanisterHttpRequestArgument, HttpHeader, HttpResponse, TransformArgs,
//! # };
//! #
//! # pub fn print(msg: &str) {
//! #    println!("{}", msg);
//! # }
//! #
//! # /// Parse the raw response body as JSON.
//! # fn parse_json(body: Vec<u8>) -> serde_json::Value {
//! #     let json_str = String::from_utf8(body).expect("Raw response is not UTF-8 encoded.");
//! #     serde_json::from_str(&json_str).expect("Failed to parse JSON from string")
//! # }
//! #
//! /// Apply a transform function to the HTTP response.
//! #[ic_cdk_macros::query]
//! fn transform(raw: TransformArgs) -> HttpResponse {
//!     let mut response = HttpResponse {
//!         status: raw.response.status.clone(),
//!         ..Default::default()
//!     };
//!     if response.status == 200 {
//!         let original = parse_json(raw.response.body);
//!
//!         // Extract the author from the JSON response.
//!         print(&format!("Before transform: {:?}", original.to_string()));
//!         let transformed = original.get("author").cloned().unwrap_or_default();
//!         print(&format!("After transform: {:?}", transformed.to_string()));
//!
//!         response.body = transformed.to_string().into_bytes();
//!     } else {
//!         print(&format!("Transform error: err = {:?}", raw));
//!     }
//!     response
//! }
//!
//! /// Create a request to the dummyjson.com API.
//! fn build_request() -> CanisterHttpRequestArgument {
//!     ic_http::create_request()
//!         .get("https://dummyjson.com/quotes/1")
//!         .header(HttpHeader {
//!             name: "User-Agent".to_string(),
//!             value: "ic-http-example-canister".to_string(),
//!         })
//!         .transform_func(transform, vec![])
//!         .build()
//! }
//!
//! /// Fetch a quote from the dummyjson.com API.
//! #[ic_cdk_macros::update]
//! async fn fetch() -> String {
//!     let request = build_request();
//!     let result = ic_http::http_request(request).await;
//!
//!     match result {
//!         Ok((response,)) => {
//!             let body = String::from_utf8(response.body).unwrap();
//!             format!("Response: {:?}", body)
//!         }
//!         Err((code, msg)) => {
//!             format!("Error: {:?} {:?}", code, msg)
//!         }
//!     }
//! }
//! ```
//!
//! ## Test
//!
//! ```rust
//! #[tokio::test]
//! async fn test_http_request_transform_body() {
//!     // Arrange
//!     let request = build_request();
//!     let mock_response = ic_http::create_response()
//!         .status(200)
//!         .body(
//!             r#"{
//!                 "id": 1,
//!                 "quote": "Life isn’t about getting and having, it’s about giving and being.",
//!                 "author": "Kevin Kruse"
//!         }"#,
//!         )
//!         .build();
//!     ic_http::mock::mock(request.clone(), mock_response);
//!
//!     // Act
//!     let (response,) = ic_http::http_request(request.clone()).await.unwrap();
//!
//!     // Assert
//!     assert_eq!(
//!         String::from_utf8(response.body).unwrap(),
//!         r#""Kevin Kruse""#.to_string()
//!     );
//!     assert_eq!(ic_http::mock::times_called(request), 1);
//! }
//! ```
//!

mod http_request;
mod request;
mod response;
mod transform;

// Export.
#[cfg(not(target_arch = "wasm32"))]
pub mod mock;

// Re-export.
pub use crate::http_request::http_request;
pub use crate::request::create_request;
pub use crate::response::create_response;
pub use crate::transform::TransformFn;

use crate::mock::{hash, Mock};
use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;
use std::collections::HashMap;
use std::sync::RwLock;

// A thread-local hashmap.
thread_local! {
    /// A thread-local hashmap of mocks.
    static MOCKS: RwLock<HashMap<String, Mock>> = RwLock::new(HashMap::new());

    /// A thread-local hashmap of transform functions.
    static TRANSFORM_FUNCTIONS: RwLock<HashMap<String, TransformFn>> = RwLock::new(HashMap::new());
}

/// Inserts the provided mock into a thread-local hashmap.
fn mock_insert(mock: Mock) {
    MOCKS.with(|cell| {
        cell.write().unwrap().insert(hash(&mock.request), mock);
    });
}

/// Returns a cloned mock from the thread-local hashmap that corresponds to the provided request.
fn mock_get(request: &CanisterHttpRequestArgument) -> Option<Mock> {
    MOCKS.with(|cell| cell.read().unwrap().get(&hash(request)).cloned())
}

/// Inserts the provided transform function into a thread-local hashmap.
fn transform_function_insert(function_name: String, func: TransformFn) {
    TRANSFORM_FUNCTIONS.with(|cell| {
        cell.write().unwrap().insert(function_name, func);
    });
}

/// Returns a cloned transform function from the thread-local hashmap.
fn transform_function_get(function_name: String) -> Option<TransformFn> {
    TRANSFORM_FUNCTIONS.with(|cell| cell.read().unwrap().get(&function_name).cloned())
}
