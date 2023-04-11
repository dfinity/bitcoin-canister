use candid::Principal;
use ic_cdk::api::management_canister::http_request::{
    HttpResponse, TransformArgs, TransformContext, TransformFunc,
};

pub type TransformFn = fn(TransformArgs) -> HttpResponse;

/// Creates a `TransformContext` from a transform function and a context.
/// Also inserts the transform function into a thread-local hashmap.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn create_transform_context(func: TransformFn, context: Vec<u8>) -> TransformContext {
    let function_name = get_function_name(func).to_string();
    crate::transform_function_insert(function_name.clone(), func);

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
    T: Fn(TransformArgs) -> HttpResponse,
{
    TransformContext::new(func, context)
}

/// Returns the name of a function as a string.
fn get_function_name<F>(_: F) -> &'static str {
    let full_name = std::any::type_name::<F>();
    match full_name.rfind(':') {
        Some(index) => &full_name[index + 1..],
        None => full_name,
    }
}
