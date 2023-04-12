use ic_cdk::api::management_canister::http_request::{
    HttpResponse, TransformArgs, TransformContext,
};

#[cfg(not(target_arch = "wasm32"))]
use {candid::Principal, ic_cdk::api::management_canister::http_request::TransformFunc};

#[cfg(not(target_arch = "wasm32"))]
pub type TransformFn = dyn Fn(TransformArgs) -> HttpResponse + 'static;

/// Creates a `TransformContext` from a transform function and a context.
/// Also inserts the transform function into a thread-local hashmap.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn create_transform_context<T>(func: T, context: Vec<u8>) -> TransformContext
where
    T: Fn(TransformArgs) -> HttpResponse + 'static,
{
    let function_name = get_function_name(&func).to_string();
    crate::transform_function_insert(function_name.clone(), Box::new(func));

    TransformContext {
        function: TransformFunc(candid::Func {
            principal: Principal::management_canister(),
            method: function_name,
        }),
        context,
    }
}

/// Creates a `TransformContext` from a transform function and a context.
#[cfg(target_arch = "wasm32")]
pub(crate) fn create_transform_context<T>(func: T, context: Vec<u8>) -> TransformContext
where
    T: Fn(TransformArgs) -> HttpResponse + 'static,
{
    TransformContext::new(func, context)
}

/// Returns the name of a function as a string.
#[cfg(not(target_arch = "wasm32"))]
fn get_function_name<F>(_: &F) -> &'static str {
    let full_name = std::any::type_name::<F>();
    match full_name.rfind(':') {
        Some(index) => &full_name[index + 1..],
        None => full_name,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// A test transform function.
    fn transform_function_1(arg: TransformArgs) -> HttpResponse {
        arg.response
    }

    /// A test transform function.
    fn transform_function_2(arg: TransformArgs) -> HttpResponse {
        arg.response
    }

    /// Inserts the provided transform function into a thread-local hashmap.
    fn insert<T>(f: T)
    where
        T: Fn(TransformArgs) -> HttpResponse + 'static,
    {
        let name = get_function_name(&f).to_string();
        crate::transform_function_insert(name, Box::new(f));
    }

    /// This test makes sure that transform function names are preserved
    /// when passing to the function.
    #[test]
    fn test_transform_function_names() {
        // Arrange.
        insert(transform_function_1);
        insert(transform_function_2);

        // Act.
        let names = crate::transform_function_names();

        // Assert.
        assert_eq!(names, vec!["transform_function_1", "transform_function_2"]);
    }
}
