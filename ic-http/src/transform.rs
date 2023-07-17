use ic_cdk::api::management_canister::http_request::{
    HttpResponse, TransformArgs, TransformContext,
};

#[cfg(not(target_arch = "wasm32"))]
use {candid::Principal, ic_cdk::api::management_canister::http_request::TransformFunc};

#[cfg(not(target_arch = "wasm32"))]
pub type TransformFn = dyn Fn(TransformArgs) -> HttpResponse + 'static;

/// Creates a `TransformContext` from a transform function and a context.
pub(crate) fn create_transform_context<T>(
    candid_function_name: &str,
    func: T,
    context: Vec<u8>,
) -> TransformContext
where
    T: Fn(TransformArgs) -> HttpResponse + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        TransformContext::from_name(candid_function_name.to_string(), context)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let method = candid_function_name.to_string();
        super::storage::transform_function_insert(method.clone(), Box::new(func));

        // crate::id() can not be called outside of canister, that's why for testing
        // it is replaced with Principal::management_canister().
        let principal = Principal::management_canister();
        TransformContext {
            function: TransformFunc(candid::Func { principal, method }),
            context,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;

    /// A test transform function.
    fn transform_function_1(arg: TransformArgs) -> HttpResponse {
        arg.response
    }

    /// A test transform function.
    fn transform_function_2(arg: TransformArgs) -> HttpResponse {
        arg.response
    }

    /// Inserts the provided transform function into a thread-local hashmap.
    fn insert<T>(name: &str, f: T)
    where
        T: Fn(TransformArgs) -> HttpResponse + 'static,
    {
        crate::storage::transform_function_insert(name.to_string(), Box::new(f));
    }

    /// This test makes sure that transform function names are preserved
    /// when passing to the function.
    #[test]
    fn test_transform_function_names() {
        // Arrange.
        insert("transform_function_1", transform_function_1);
        insert("transform_function_2", transform_function_2);

        // Act.
        let names = crate::mock::registered_transform_function_names();

        // Assert.
        assert_eq!(names, vec!["transform_function_1", "transform_function_2"]);
    }

    /// Transform function which intentionally creates a new request passing
    /// itself as the target transform function.
    fn transform_function_with_overwrite(arg: TransformArgs) -> HttpResponse {
        create_request_with_transform();
        arg.response
    }

    /// Creates a request with a transform function which overwrites itself.
    fn create_request_with_transform() -> CanisterHttpRequestArgument {
        crate::request::create_request()
            .url("https://www.example.com")
            .transform_func(
                "transform_function_with_overwrite",
                transform_function_with_overwrite,
                vec![],
            )
            .build()
    }

    // IMPORTANT: If this test hangs check the implementation of inserting
    // transform function to the thread-local storage.
    //
    // This test simulates the case when transform function tries to
    // rewrite itself in a thread-local storage while it is being executed.
    // This may lead to a hang if the insertion to the thread-local storage
    // is not written properly.
    #[tokio::test]
    async fn test_transform_function_call_without_a_hang() {
        // Arrange
        let request = create_request_with_transform();
        let mock_response = crate::response::create_response().build();
        crate::mock::mock(request.clone(), mock_response);

        // Act
        let (response,) = crate::mock::http_request(request.clone()).await.unwrap();

        // Assert
        assert_eq!(response.status, 200);
        assert_eq!(crate::mock::times_called(request), 1);
        assert_eq!(
            crate::mock::registered_transform_function_names(),
            vec!["transform_function_with_overwrite"]
        );
    }
}
