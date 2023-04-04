# ic-http-mock

`ic-http-mock` is a library for mocking HTTPS outcalls on the Internet Computer.

## References

- [Integrations](https://internetcomputer.org/docs/current/developer-docs/integrations/)
- [HTTPS Outcalls](https://internetcomputer.org/docs/current/developer-docs/integrations/http_requests/)
- [IC method http_request](https://internetcomputer.org/docs/current/references/ic-interface-spec#ic-http_request)
- [Transformation Function](https://internetcomputer.org/docs/current/developer-docs/integrations/http_requests/http_requests-how-it-works#transformation-function)

## Getting Started

Add `ic-http-mock` to your `Cargo.toml` and start mocking:

```rust
#[tokio::test]
async fn test_http_request_transform_body() {
    // Arrange
    const ORIGINAL_BODY: &str = "original body";
    const TRANSFORMED_BODY: &str = "transformed body";
    fn transform(_arg: TransformArgs) -> HttpResponse {
        create_response().body(TRANSFORMED_BODY).build()
    }
    let request = create_request()
        .get("https://example.com")
        .transform(create_transform_context(transform, vec![]))
        .build();
    let mocked_response = create_response()
        .status(200)
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
```

If you have a method that creates a request inside the canister make sure to create a conditional wrapper around building `TransformContext::new(func, context)`. Otherwise it can cause the test failing with `canister_self_size should only be called inside canisters.` error.

Canister code:

```rust
/// Apply a transform function to the HTTP response.
#[cfg(not(target_arch = "wasm32"))]
pub fn transform_context_wrapper(func: TransformFn, context: Vec<u8>) -> TransformContext {
    ic_http::create_transform_context(func, context)
}

/// Apply a transform function to the HTTP response.
#[cfg(target_arch = "wasm32")]
pub fn transform_context_wrapper<T>(func: T, context: Vec<u8>) -> TransformContext
where
    T: Fn(TransformArgs) -> HttpResponse,
{
    TransformContext::new(func, context)
}

/// Create a request to the dummyjson.com API.
fn build_request() -> CanisterHttpRequestArgument {
    let host = "dummyjson.com";
    let url = format!("https://{host}/quotes/1");
    ic_http::create_request()
        .get(&url)
        .header(HttpHeader {
            name: "User-Agent".to_string(),
            value: "ic-http-mock-example-canister".to_string(),
        })
        .transform(transform_context_wrapper(transform, vec![]))
        .build()
}

/// Wrapper around the http_request function.
#[cfg(not(target_arch = "wasm32"))]
pub async fn http_request_wrapper(
    arg: &CanisterHttpRequestArgument,
) -> CallResult<(HttpResponse,)> {
    ic_http::http_request(arg).await
}

/// Wrapper around the http_request function.
#[cfg(target_arch = "wasm32")]
pub async fn http_request_wrapper(
    arg: &CanisterHttpRequestArgument,
) -> CallResult<(HttpResponse,)> {
    ic_cdk::api::management_canister::http_request::http_request(arg.clone()).await
}
```

Corresponding test:

```rust
#[tokio::test]
async fn test_http_request_transform_body() {
    // Arrange
    let request = build_request();
    let mocked_response = ic_http::create_response()
        .status(200)
        .body(
            r#"{
            "id": 1,
            "quote": "Life isn’t about getting and having, it’s about giving and being.",
            "author": "Kevin Kruse"
        }"#,
        )
        .build();
    ic_http::mock(&request, &mocked_response);

    // Act
    let (response,) = http_request_wrapper(&request).await.unwrap();

    // Assert
    assert_eq!(
        String::from_utf8(response.body).unwrap(),
        r#""Kevin Kruse""#.to_string()
    );
    assert_eq!(ic_http::times_called(&request), 1);
}
```

## Examples

See usage example on a simple canister [here](./example_canister/)

## Testing

```bash
# Crate tests.
$ cargo test

# Example Canister tests.
$ cd example_canister
$ cargo test
```
