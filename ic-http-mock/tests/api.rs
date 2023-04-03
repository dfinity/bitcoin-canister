use ic_http_mock::*;
use std::time::{Duration, Instant};

const STATUS_CODE_OK: u64 = 200;
const STATUS_CODE_NOT_FOUND: u64 = 404;

#[tokio::test]
async fn test_http_request_no_transform() {
    // Arrange
    let body = "some text";
    let request = create_request().get("https://example.com").build();
    let mock_response = create_response().status(STATUS_CODE_OK).body(body).build();
    mock(&request, &mock_response);

    // Act
    let (response,) = http_request(&request).await.unwrap();

    // Assert
    assert_eq!(response.status, candid::Nat::from(STATUS_CODE_OK));
    assert_eq!(response.body, body.to_string().as_bytes().to_vec());
    assert_eq!(times_called(&request), 1);
}

#[tokio::test]
async fn test_http_request_called_several_times() {
    // Arrange
    let calls = 3;
    let body = "some text";
    let request = create_request().get("https://example.com").build();
    let mock_response = create_response().status(STATUS_CODE_OK).body(body).build();
    mock(&request, &mock_response);

    // Act
    for _ in 0..calls {
        let (response,) = http_request(&request).await.unwrap();
        assert_eq!(response.status, candid::Nat::from(STATUS_CODE_OK));
        assert_eq!(response.body, body.to_string().as_bytes().to_vec());
    }

    // Assert
    assert_eq!(times_called(&request), calls);
}

#[tokio::test]
async fn test_http_request_transform_status() {
    // Arrange
    fn transform(_arg: TransformArgs) -> HttpResponse {
        create_response().status(STATUS_CODE_NOT_FOUND).build()
    }
    let request = create_request()
        .get("https://example.com")
        .transform(create_transform_context(transform, vec![]))
        .build();
    let mock_response = create_response()
        .status(STATUS_CODE_OK)
        .body("some text")
        .build();
    mock(&request, &mock_response);

    // Act
    let (response,) = http_request(&request).await.unwrap();

    // Assert
    assert_ne!(response.status, candid::Nat::from(STATUS_CODE_OK));
    assert_eq!(response.status, candid::Nat::from(STATUS_CODE_NOT_FOUND));
    assert_eq!(times_called(&request), 1);
}

#[tokio::test]
async fn test_http_request_transform_body() {
    // Arrange
    const ORIGINAL_BODY: &str = "original body";
    const TRANSFORMED_BODY: &str = "transformed body";
    fn transform(_arg: TransformArgs) -> HttpResponse {
        create_response().body(TRANSFORMED_BODY).build()
    }
    let request = create_request()
        .get("https://dummyjson.com/todos/1")
        .transform(create_transform_context(transform, vec![]))
        .build();
    let mocked_response = create_response()
        .status(STATUS_CODE_OK)
        .body(ORIGINAL_BODY)
        .build();
    mock(&request, &mocked_response);

    // Act
    let (response,) = http_request(&request).await.unwrap();

    // Assert
    assert_ne!(response.body, ORIGINAL_BODY.as_bytes().to_vec());
    assert_eq!(response.body, TRANSFORMED_BODY.as_bytes().to_vec());
    assert_eq!(times_called(&request), 1);
}

#[tokio::test]
async fn test_http_request_max_response_bytes_ok() {
    // Arrange
    let max_response_bytes = 3;
    let body_small_enough = "123";
    let request = create_request()
        .get("https://example.com")
        .max_response_bytes(max_response_bytes)
        .build();
    let mocked_response = create_response()
        .status(STATUS_CODE_OK)
        .body(body_small_enough)
        .build();
    mock(&request, &mocked_response);

    // Act
    let result = http_request(&request).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(times_called(&request), 1);
}

#[tokio::test]
async fn test_http_request_max_response_bytes_error() {
    // Arrange
    let max_response_bytes = 3;
    let body_too_big = "1234";
    let request = create_request()
        .get("https://example.com")
        .max_response_bytes(max_response_bytes)
        .build();
    let mocked_response = create_response()
        .status(STATUS_CODE_OK)
        .body(body_too_big)
        .build();
    mock(&request, &mocked_response);

    // Act
    let result = http_request(&request).await;

    // Assert
    assert!(result.is_err());
    assert_eq!(times_called(&request), 1);
}

#[tokio::test]
async fn test_http_request_sequentially() {
    // Arrange
    let request_a = create_request().get("a").build();
    let request_b = create_request().get("b").build();
    let request_c = create_request().get("c").build();
    let mocked_response = create_response().status(STATUS_CODE_OK).build();
    mock_with_delay(&request_a, &mocked_response, Duration::from_millis(100));
    mock_with_delay(&request_b, &mocked_response, Duration::from_millis(200));
    mock_with_delay(&request_c, &mocked_response, Duration::from_millis(300));

    // Act
    let start = Instant::now();
    let _ = http_request(&request_a).await;
    let _ = http_request(&request_b).await;
    let _ = http_request(&request_c).await;
    println!("All finished after {} s", start.elapsed().as_secs_f32());

    // Assert
    assert!(start.elapsed() > Duration::from_millis(500));
    assert_eq!(times_called(&request_a), 1);
    assert_eq!(times_called(&request_b), 1);
    assert_eq!(times_called(&request_c), 1);
}

#[tokio::test]
async fn test_http_request_concurrently() {
    // Arrange
    let request_a = create_request().get("a").build();
    let request_b = create_request().get("b").build();
    let request_c = create_request().get("c").build();
    let mocked_response = create_response().status(STATUS_CODE_OK).build();
    mock_with_delay(&request_a, &mocked_response, Duration::from_millis(100));
    mock_with_delay(&request_b, &mocked_response, Duration::from_millis(200));
    mock_with_delay(&request_c, &mocked_response, Duration::from_millis(300));

    // Act
    let start = Instant::now();
    let futures = vec![
        http_request(&request_a),
        http_request(&request_b),
        http_request(&request_c),
    ];
    futures::future::join_all(futures).await;
    println!("All finished after {} s", start.elapsed().as_secs_f32());

    // Assert
    assert!(start.elapsed() < Duration::from_millis(500));
    assert_eq!(times_called(&request_a), 1);
    assert_eq!(times_called(&request_b), 1);
    assert_eq!(times_called(&request_c), 1);
}
