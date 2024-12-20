use crate::{
    mock::{hash, Mock},
    transform::TransformFn,
};
use ic_cdk::api::management_canister::http_request::CanisterHttpRequestArgument;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use std::collections::HashMap;
use std::sync::RwLock;

// A thread-local hashmap.
thread_local! {
    /// A thread-local hashmap of mocks.
    static MOCKS: RwLock<HashMap<String, Mock>> = RwLock::new(HashMap::new());

    /// A thread-local hashmap of transform functions.
    static TRANSFORM_FUNCTIONS: RwLock<HashMap<String, Box<TransformFn>>> = RwLock::new(HashMap::new());
}

/// Inserts the provided mock into a thread-local hashmap.
pub(crate) fn mock_insert(mock: Mock) {
    MOCKS.with(|cell| {
        cell.write().unwrap().insert(hash(&mock.request), mock);
    });
}

/// Returns a cloned mock from the thread-local hashmap that corresponds to the provided request.
pub(crate) fn mock_get(request: &CanisterHttpRequestArgument) -> Option<Mock> {
    MOCKS.with(|cell| cell.read().unwrap().get(&hash(request)).cloned())
}

/// Inserts the provided transform function into a thread-local hashmap.
/// If a transform function with the same name already exists, it is not inserted.
pub(crate) fn transform_function_insert(name: String, func: Box<TransformFn>) {
    TRANSFORM_FUNCTIONS.with(|cell| {
        // This is a workaround to prevent the transform function from being
        // overridden while it is being executed.
        if cell.read().unwrap().get(&name).is_none() {
            cell.write().unwrap().insert(name, func);
        }
    });
}

/// Executes the transform function that corresponds to the provided name.
pub(crate) fn transform_function_call(name: String, arg: TransformArgs) -> Option<HttpResponse> {
    TRANSFORM_FUNCTIONS.with(|cell| cell.read().unwrap().get(&name).map(|f| f(arg)))
}

/// Returns a sorted list of transform function names.
/// This is used for testing.
pub(crate) fn transform_function_names() -> Vec<String> {
    TRANSFORM_FUNCTIONS.with(|cell| {
        let mut names: Vec<String> = cell.read().unwrap().keys().cloned().collect();
        names.sort();
        names
    })
}
