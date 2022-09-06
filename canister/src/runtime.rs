//! Wrapper methods around the IC0 API and the canister's asynchronous calls.
//!
//! Alternative implementations are available in non-wasm environments to
//! facilitate testing.
use crate::types::{GetSuccessorsCompleteResponse, GetSuccessorsRequest, GetSuccessorsResponse};
use ic_cdk::{api::call::CallResult, export::Principal};
use std::cell::RefCell;
use std::future::Future;

#[cfg(not(target_arch = "wasm32"))]
const INSTRUCTIONS_LIMIT: u64 = 5_000_000_000;

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

    static PERFORMANCE_COUNTER: RefCell<u64> = RefCell::new(0);

    static PERFORMANCE_COUNTER_STEP: RefCell<u64> = RefCell::new(0);
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
            .unwrap_or(GetSuccessorsResponse::Complete(
                GetSuccessorsCompleteResponse {
                    blocks: vec![],
                    next: vec![],
                },
            ))
    }),)))
}

/// Sets a (mock) response to return whenever `call_get_successors` is invoked.
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub fn set_successors_response(response: GetSuccessorsResponse) {
    GET_SUCCESSORS_RESPONSE.with(|e| e.replace(Some(response)));
}

#[cfg(target_arch = "wasm32")]
pub fn performance_counter() -> u64 {
    ic_cdk::api::performance_counter(0)
}

/// Increments a mock performance counter and panics whenever the instruction limit is exceeded.
#[cfg(not(target_arch = "wasm32"))]
pub fn performance_counter() -> u64 {
    PERFORMANCE_COUNTER.with(|pc| {
        *pc.borrow_mut() += PERFORMANCE_COUNTER_STEP.with(|ps| *ps.borrow());

        if *pc.borrow() > INSTRUCTIONS_LIMIT {
            panic!("instructions limit exceeded");
        }

        *pc.borrow()
    })
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub fn performance_counter_reset() {
    PERFORMANCE_COUNTER.with(|pc| *pc.borrow_mut() = 0)
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub fn set_performance_counter_step(step_size: u64) {
    PERFORMANCE_COUNTER_STEP.with(|pc| *pc.borrow_mut() = step_size)
}
