use ic_cdk::api::management_canister::http_request::{CanisterHttpRequestArgument, HttpHeader};

#[cfg(not(target_arch = "wasm32"))]
use ic_http::{HttpResponse, TransformArgs};

#[cfg(target_arch = "wasm32")]
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};

#[cfg(target_arch = "wasm32")]
pub fn print(msg: &str) {
    ic_cdk::api::print(msg);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn print(msg: &str) {
    println!("{}", msg);
}

/// Apply a transform function to the HTTP response.
#[ic_cdk_macros::query]
fn transform(raw: TransformArgs) -> HttpResponse {
    let mut response = HttpResponse {
        status: raw.response.status.clone(),
        ..Default::default()
    };
    if response.status == 200 {
        let json_str =
            String::from_utf8(raw.response.body).expect("Raw response is not UTF-8 encoded.");
        print(&format!("Before transform body: {:?}", json_str));
        let json: serde_json::Value =
            serde_json::from_str(&json_str).expect("Failed to parse JSON from string");
        // Extract the author from the JSON response.
        let transformed = json
            .get("author")
            .cloned()
            .map(|x| x.to_string())
            .unwrap_or_default();
        print(&format!("After transform body: {:?}", transformed));
        response.body = transformed.into_bytes();
    } else {
        print(&format!("Transform error: err = {:?}", raw));
    }
    response
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
        .transform(ic_http::create_transform_context(transform, vec![]))
        .build()
}

/// Fetch a quote from the dummyjson.com API.
#[ic_cdk_macros::update]
async fn fetch() -> String {
    let request = build_request();
    print(&format!("Request: {:?}", request));
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

#[cfg(test)]
mod test {
    use super::*;

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
}
