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
//!     fn transform(_arg: TransformArgs) -> HttpResponse {
//!         create_response().body(TRANSFORMED_BODY).build()
//!     }
//!     let request = create_request()
//!         .get("https://example.com")
//!         .transform(create_transform_context(transform, vec![]))
//!         .build();
//!     let mocked_response = create_response()
//!         .status(200)
//!         .body(ORIGINAL_BODY)
//!         .build();
//!     mock(&request, &mocked_response);
//!
//!     // Act
//!     let (response,) = http_request(&request).await.unwrap();
//!
//!     // Assert
//!     assert_ne!(response.body, ORIGINAL_BODY.as_bytes().to_vec());
//!     assert_eq!(response.body, TRANSFORMED_BODY.as_bytes().to_vec());
//!     assert_eq!(times_called(&request), 1);
//! }
//! ```
//!

mod mock;
mod request;
mod response;
mod transform;

// Re-export.
pub use crate::mock::http_request;
pub use crate::mock::mock;
pub use crate::mock::mock_with_delay;
pub use crate::mock::times_called;
pub use crate::request::create_request;
pub use crate::response::create_response;
pub use crate::transform::create_transform_context;
pub use crate::transform::TransformFn;
pub use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;
pub use ic_cdk::api::management_canister::http_request::HttpMethod;
pub use ic_cdk::api::management_canister::http_request::HttpResponse;
pub use ic_cdk::api::management_canister::http_request::TransformArgs;
pub use ic_cdk::api::management_canister::http_request::TransformContext;
