//! A module with wrapper methods around the IC0 API, with alternative implementations
//! available in non-wasm environments to facilitate testing.
use crate::types::{GetSuccessorsRequest, GetSuccessorsResponse};
use ic_cdk::{api::call::CallResult, export::Principal};
use std::cell::RefCell;
use std::future::Future;

#[cfg(target_arch = "wasm32")]
pub fn print(msg: &str) {
    ic_cdk::api::print(msg);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn print(msg: &str) {
    println!("{}", msg);
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    // A mock response to return when `call_get_successors` is invoked.
    static GET_SUCCESSORS_RESPONSE: RefCell<Option<GetSuccessorsResponse>> = RefCell::new(None);
}

#[cfg(target_arch = "wasm32")]
pub fn call_get_successors(
    id: Principal,
    request: GetSuccessorsRequest,
) -> impl Future<Output = CallResult<(GetSuccessorsResponse,)>> {
    return ic_cdk::api::call::call(id, "bitcoin_get_successors", (request,));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn call_get_successors(
    _id: Principal,
    _request: GetSuccessorsRequest,
) -> impl Future<Output = CallResult<(GetSuccessorsResponse,)>> {
    std::future::ready(Ok((GET_SUCCESSORS_RESPONSE.with(|e| {
        e.borrow_mut()
            .take()
            .expect("no mock GetSuccessorsResponse provided")
    }),)))
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
/// Sets a (mock) response to return whenever `call_get_successors` is invoked.
pub fn set_successors_response(response: GetSuccessorsResponse) {
    GET_SUCCESSORS_RESPONSE.with(|e| e.replace(Some(response)));
}
