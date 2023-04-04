//!
//! `ic-http-mock` is a library for mocking HTTPS outcalls on the Internet Computer.
//!
//! # Example
//! ```rust
//! #[tokio::test]
//! async fn test_http_request_transform_body() {
//!     // Arrange
//!     const ORIGINAL_BODY: &str = "original body";
//!     const TRANSFORMED_BODY: &str = "transformed body";
//!     fn transform(_arg: ic_http::TransformArgs) -> ic_http::HttpResponse {
//!         ic_http::create_response().body(TRANSFORMED_BODY).build()
//!     }
//!     let request = ic_http::create_request()
//!         .get("https://example.com")
//!         .transform(ic_http::create_transform_context(transform, vec![]))
//!         .build();
//!     let mocked_response = ic_http::create_response()
//!         .status(200)
//!         .body(ORIGINAL_BODY)
//!         .build();
//!     ic_http::mock::mock(request.clone(), mocked_response);
//!
//!     // Act
//!     let (response,) = ic_http::http_request(request.clone()).await.unwrap();
//!
//!     // Assert
//!     assert_ne!(response.body, ORIGINAL_BODY.as_bytes().to_vec());
//!     assert_eq!(response.body, TRANSFORMED_BODY.as_bytes().to_vec());
//!     assert_eq!(ic_http::mock::times_called(request), 1);
//! }
//! ```
//!

pub mod http_request;
pub mod mock;
mod request;
mod response;
mod transform;

// Re-export.
pub use crate::http_request::http_request;
pub use crate::request::create_request;
pub use crate::response::create_response;
pub use crate::transform::create_transform_context;
pub use crate::transform::TransformFn;
pub use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;
pub use ic_cdk::api::management_canister::http_request::HttpMethod;
pub use ic_cdk::api::management_canister::http_request::HttpResponse;
pub use ic_cdk::api::management_canister::http_request::TransformArgs;
pub use ic_cdk::api::management_canister::http_request::TransformContext;
