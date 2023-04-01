mod fetch;
mod mock;
mod transform;
mod types;

pub use fetch::fetch;
pub use mock::mock;
pub use mock::mock_with_delay;

pub use ic_cdk::api::management_canister::http_request::HttpMethod;
pub use ic_cdk::api::management_canister::http_request::HttpResponse;
pub use ic_cdk::api::management_canister::http_request::TransformArgs;
pub use transform::TransformContextBuilder;
pub use transform::TransformFn;
pub use types::HttpRequestBuilder;
pub use types::HttpResponseBuilder;

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;
    use std::time::Instant;

    #[tokio::test]
    async fn test_fetch_no_transform() {
        let request = HttpRequestBuilder::new()
            .url("https://example.com")
            .method(HttpMethod::GET)
            .build();
        let mocked_response = HttpResponseBuilder::new()
            .status(200)
            .body("some text")
            .build();
        mock(&request, &mocked_response);

        let (response,) = fetch(request).await.unwrap();

        assert_eq!(response, mocked_response);
    }

    #[tokio::test]
    async fn test_fetch_transform_status() {
        let original_status = 200;
        fn transformed_status() -> i32 {
            404
        }
        fn transform(_arg: TransformArgs) -> HttpResponse {
            HttpResponseBuilder::new()
                .status(transformed_status())
                .build()
        }
        let request = HttpRequestBuilder::new()
            .url("https://example.com")
            .method(HttpMethod::GET)
            .transform(transform, vec![])
            .build();
        let mocked_response = HttpResponseBuilder::new()
            .status(original_status)
            .body("some text")
            .build();
        mock(&request, &mocked_response);

        let (response,) = fetch(request).await.unwrap();

        assert_ne!(response.status, candid::Nat::from(original_status));
        assert_eq!(response.status, candid::Nat::from(transformed_status()));
    }

    #[tokio::test]
    async fn test_fetch_transform_body() {
        let original_body = "original body";
        fn transformed_body() -> String {
            "transformed body".to_string()
        }
        fn transform(_arg: TransformArgs) -> HttpResponse {
            HttpResponseBuilder::new().body(&transformed_body()).build()
        }
        let request = HttpRequestBuilder::new()
            .url("https://dummyjson.com/todos/1")
            .transform(transform, vec![])
            .build();
        let mocked_response = HttpResponseBuilder::new()
            .status(200)
            .body(original_body)
            .build();
        mock(&request, &mocked_response);

        let (response,) = fetch(request).await.unwrap();

        assert_ne!(response.body, original_body.as_bytes().to_vec());
        assert_eq!(response.body, transformed_body().as_bytes().to_vec());
    }

    #[tokio::test]
    async fn test_fetch_max_response_bytes_ok() {
        let max_response_bytes = 3;
        let body_small_enough = "123";
        let request = HttpRequestBuilder::new()
            .url("https://example.com")
            .method(HttpMethod::GET)
            .max_response_bytes(max_response_bytes)
            .build();
        let mocked_response = HttpResponseBuilder::new()
            .status(200)
            .body(body_small_enough)
            .build();
        mock(&request, &mocked_response);

        let result = fetch(request).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_max_response_bytes_error() {
        let max_response_bytes = 3;
        let body_too_big = "1234";
        let request = HttpRequestBuilder::new()
            .url("https://example.com")
            .method(HttpMethod::GET)
            .max_response_bytes(max_response_bytes)
            .build();
        let mocked_response = HttpResponseBuilder::new()
            .status(200)
            .body(body_too_big)
            .build();
        mock(&request, &mocked_response);

        let result = fetch(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_sequentially() {
        let request_a = HttpRequestBuilder::new().url("a").build();
        let request_b = HttpRequestBuilder::new().url("b").build();
        let request_c = HttpRequestBuilder::new().url("c").build();

        let mocked_response = HttpResponseBuilder::new().status(200).build();
        mock_with_delay(&request_a, &mocked_response, Duration::from_millis(100));
        mock_with_delay(&request_b, &mocked_response, Duration::from_millis(200));
        mock_with_delay(&request_c, &mocked_response, Duration::from_millis(300));

        let start = Instant::now();
        let _ = fetch(request_a).await;
        let _ = fetch(request_b).await;
        let _ = fetch(request_c).await;
        println!("All finished after {} s", start.elapsed().as_secs_f32());

        assert!(start.elapsed() > Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_fetch_concurrently() {
        let request_a = HttpRequestBuilder::new().url("a").build();
        let request_b = HttpRequestBuilder::new().url("b").build();
        let request_c = HttpRequestBuilder::new().url("c").build();

        let mocked_response = HttpResponseBuilder::new().status(200).build();
        mock_with_delay(&request_a, &mocked_response, Duration::from_millis(100));
        mock_with_delay(&request_b, &mocked_response, Duration::from_millis(200));
        mock_with_delay(&request_c, &mocked_response, Duration::from_millis(300));

        let start = Instant::now();
        let futures = vec![fetch(request_a), fetch(request_b), fetch(request_c)];
        futures::future::join_all(futures).await;
        println!("All finished after {} s", start.elapsed().as_secs_f32());

        assert!(start.elapsed() < Duration::from_millis(500));
    }
}
