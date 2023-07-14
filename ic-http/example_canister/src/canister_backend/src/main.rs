use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpHeader, HttpResponse, TransformArgs,
};

const ZERO_CYCLES: u128 = 0;

/// Print a message to the console.
pub fn print(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    ic_cdk::api::print(msg);

    #[cfg(not(target_arch = "wasm32"))]
    println!("{}", msg);
}

/// Parse the raw response body as JSON.
fn parse_json(body: Vec<u8>) -> serde_json::Value {
    let json_str = String::from_utf8(body).expect("Raw response is not UTF-8 encoded.");
    serde_json::from_str(&json_str).expect("Failed to parse JSON from string")
}

/// Apply a transform function to the quote HTTP response.
#[ic_cdk_macros::query]
fn transform_quote(raw: TransformArgs) -> HttpResponse {
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

/// Create a request to the dummyjson.com API for quotes.
fn build_quote_request(url: &str) -> CanisterHttpRequestArgument {
    ic_http::create_request()
        .get(url)
        .header(HttpHeader {
            name: "User-Agent".to_string(),
            value: "ic-http-example-canister".to_string(),
        })
        .transform_func("transform_quote", transform_quote, vec![])
        .build()
}

/// Fetch data by making an HTTP request.
async fn fetch(request: CanisterHttpRequestArgument) -> String {
    let result = ic_http::http_request(request, ZERO_CYCLES).await;

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

/// Fetch a quote from the dummyjson.com API.
#[ic_cdk_macros::update]
async fn fetch_quote() -> String {
    let request = build_quote_request("https://dummyjson.com/quotes/1");
    fetch(request).await
}

fn main() {}

#[cfg(test)]
mod test {
    use super::*;
    use ic_cdk::api::call::RejectionCode;

    #[tokio::test]
    async fn test_http_request_transform_body_quote() {
        // Arrange.
        let request = build_quote_request("https://dummyjson.com/quotes/1");
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

        // Act.
        let (response,) = ic_http::http_request(request.clone(), ZERO_CYCLES)
            .await
            .unwrap();

        // Assert.
        assert_eq!(
            String::from_utf8(response.body).unwrap(),
            r#""Kevin Kruse""#.to_string()
        );
        assert_eq!(ic_http::mock::times_called(request), 1);
    }

    #[tokio::test]
    async fn test_http_request_transform_body_quote_404() {
        // Arrange.
        let request = build_quote_request("https://dummyjson.com/quotes/1");
        let mock_response = ic_http::create_response().status(404).build();
        ic_http::mock::mock(request.clone(), mock_response);

        // Act.
        let (response,) = ic_http::http_request(request.clone(), ZERO_CYCLES)
            .await
            .unwrap();

        // Assert.
        assert_eq!(response.status, 404);
        assert_eq!(ic_http::mock::times_called(request), 1);
    }

    #[tokio::test]
    async fn test_http_request_transform_body_quote_error() {
        // Arrange.
        let request = build_quote_request("https://dummyjson.com/quotes/1");
        let mock_error = (RejectionCode::SysFatal, "system fatal error".to_string());
        ic_http::mock::mock_error(request.clone(), mock_error);

        // Act.
        let result = ic_http::http_request(request.clone(), ZERO_CYCLES).await;

        // Assert.
        assert_eq!(
            result,
            Err((RejectionCode::SysFatal, "system fatal error".to_string()))
        );
        assert_eq!(ic_http::mock::times_called(request), 1);
    }
}
