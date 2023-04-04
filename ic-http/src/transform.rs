use candid::Principal;
use ic_cdk::api::management_canister::http_request::{
    HttpResponse, TransformArgs, TransformContext, TransformFunc,
};
use std::cell::RefCell;
use std::collections::HashMap;

pub type TransformFn = fn(TransformArgs) -> HttpResponse;

// A thread-local hashmap that stores transform functions.
thread_local! {
    static TRANSFORM_FUNCTIONS: RefCell<HashMap<String, TransformFn>> = RefCell::default();
}

/// Inserts the provided transform function into a thread-local hashmap.
fn registry_insert(function_name: String, func: TransformFn) {
    TRANSFORM_FUNCTIONS.with(|cell| cell.borrow_mut().insert(function_name, func));
}

/// Returns a cloned transform function from the thread-local hashmap.
pub fn registry_get(function_name: String) -> Option<TransformFn> {
    TRANSFORM_FUNCTIONS.with(|cell| cell.borrow().get(&function_name).copied())
}

/// Returns the name of a function as a string.
fn get_function_name<F>(_: F) -> &'static str {
    let full_name = std::any::type_name::<F>();
    match full_name.rfind(':') {
        Some(index) => &full_name[index + 1..],
        None => full_name,
    }
}

/// Creates a `TransformContext` from a transform function and a context.
/// Also inserts the transform function into a thread-local hashmap.
#[cfg(not(target_arch = "wasm32"))]
pub fn create_transform_context(func: TransformFn, context: Vec<u8>) -> TransformContext {
    let function_name = get_function_name(func).to_string();
    registry_insert(function_name.clone(), func);

    TransformContext {
        function: TransformFunc(candid::Func {
            principal: Principal::anonymous(),
            method: function_name,
        }),
        context,
    }
}

/// Creates a `TransformContext` from a transform function and a context.
#[cfg(target_arch = "wasm32")]
pub fn create_transform_context<T>(func: T, context: Vec<u8>) -> TransformContext
where
    T: Fn(TransformArgs) -> HttpResponse,
{
    TransformContext::new(func, context)
}
