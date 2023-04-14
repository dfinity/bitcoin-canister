# ic-http

`ic-http` offers a simplified API to make HTTP outcalls on the Internet Computer with an extra convenience of mocking them in tests.

## References

- [Integrations](https://internetcomputer.org/docs/current/developer-docs/integrations/)
- [HTTPS Outcalls](https://internetcomputer.org/docs/current/developer-docs/integrations/http_requests/)
- [IC method http_request](https://internetcomputer.org/docs/current/references/ic-interface-spec#ic-http_request)
- [Transformation Function](https://internetcomputer.org/docs/current/developer-docs/integrations/http_requests/http_requests-how-it-works#transformation-function)

## Getting Started

Usage:
- Add `ic-http` to your `Cargo.toml`
- Create a canister
- Build a request using the `ic_http::create_request`
  - If necessary, provide a transform function
- Make an HTTP request using `ic_http::http_request`
- Test with mock data by using `ic_http::mock::mock`, `ic_http::mock::mock_err`, etc.

Canister:

```rust
/// Apply a transform function to the HTTP response.
#[ic_cdk_macros::query]
fn transform(raw: TransformArgs) -> HttpResponse {
    let mut response = HttpResponse {
        status: raw.response.status.clone(),
        ..Default::default()
    };
    if response.status == 200 {
        let original = parse_json(raw.response.body);

        // Extract the author from the JSON response.
        print(&format!("Before transform: {:?}", original.to_string()));
        let transformed = original.get("author").cloned().unwrap_or_default();
        print(&format!("After transform: {:?}", transformed.to_string()));

        response.body = transformed.to_string().into_bytes();
    } else {
        print(&format!("Transform error: err = {:?}", raw));
    }
    response
}

/// Create a request to the dummyjson.com API.
fn build_request() -> CanisterHttpRequestArgument {
    ic_http::create_request()
        .get("https://dummyjson.com/quotes/1")
        .header(HttpHeader {
            name: "User-Agent".to_string(),
            value: "ic-http-example-canister".to_string(),
        })
        .transform_func(transform, vec![])
        .build()
}

/// Fetch a quote from the dummyjson.com API.
#[ic_cdk_macros::update]
async fn fetch() -> String {
    let request = build_request();
    let result = ic_http::http_request(request).await;

    match result {
        Ok((response,)) => {
            let body = String::from_utf8(response.body).unwrap();
            format!("Response: {:?}", body)
        }
        Err((code, msg)) => {
            format!("Error: {:?} {:?}", code, msg)
        }
    }
}
```

Test:

```rust
#[tokio::test]
async fn test_http_request_transform_body() {
    // Arrange
    let request = build_request();
    let mock_response = ic_http::create_response()
        .status(200)
        .body(
            r#"{
                "id": 1,
                "quote": "Life isn’t about getting and having, it’s about giving and being.",
                "author": "Kevin Kruse"
        }"#,
        )
        .build();
    ic_http::mock::mock(request.clone(), mock_response);

    // Act
    let (response,) = ic_http::http_request(request.clone()).await.unwrap();

    // Assert
    assert_eq!(
        String::from_utf8(response.body).unwrap(),
        r#""Kevin Kruse""#.to_string()
    );
    assert_eq!(ic_http::mock::times_called(request), 1);
}
```

## Examples

- Simple canister [here](./example_canister/)
- Some API tests [here](./tests/api.rs)

## Testing

```bash
# Crate tests.
$ cargo test

# Example Canister tests.
$ cd example_canister
$ cargo test
```
