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
