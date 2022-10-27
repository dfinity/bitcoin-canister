//! Wrapper methods around the IC0 API and the canister's asynchronous calls.
//!
//! Alternative implementations are available in non-wasm environments to
//! facilitate testing.
use crate::types::{GetSuccessorsRequest, GetSuccessorsResponse};
use ic_cdk::{api::call::CallResult, export::Principal};
#[cfg(not(target_arch = "wasm32"))]
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
    // Mock responses to return when `call_get_successors` is invoked.
    // Responses are returned in the order provided.
    static GET_SUCCESSORS_RESPONSES: RefCell<Vec<GetSuccessorsResponse>> = RefCell::new(Vec::default());

    static GET_SUCCESSORS_RESPONSES_INDEX: RefCell<usize> = RefCell::new(0);

    static PERFORMANCE_COUNTER: RefCell<u64> = RefCell::new(0);

    static PERFORMANCE_COUNTER_STEP: RefCell<u64> = RefCell::new(0);

    static CYCLES_BALANCE: RefCell<u64> = RefCell::new(0);
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
    use crate::types::GetSuccessorsCompleteResponse;

    let response = GET_SUCCESSORS_RESPONSES.with(|responses| {
        // Get the response at the current index.
        GET_SUCCESSORS_RESPONSES_INDEX.with(|i| {
            let response = responses
                .borrow()
                .get(*i.borrow())
                .unwrap_or(&GetSuccessorsResponse::Complete(
                    GetSuccessorsCompleteResponse {
                        blocks: vec![],
                        next: vec![],
                    },
                ))
                .clone();

            // Increment index.
            *i.borrow_mut() += 1;

            response
        })
    });

    std::future::ready(Ok((response,)))
}

/// Sets a (mock) response to return whenever `call_get_successors` is invoked.
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub fn set_successors_response(response: GetSuccessorsResponse) {
    set_successors_responses(vec![response]);
}

/// Sets (mock) responses to return whenever `call_get_successors` is invoked.
/// Responses are returned in order.
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub fn set_successors_responses(responses: Vec<GetSuccessorsResponse>) {
    GET_SUCCESSORS_RESPONSES.with(|e| e.replace(responses));
    GET_SUCCESSORS_RESPONSES_INDEX.with(|e| e.replace(0));
}

/// In production this is equivalent to `performance_counter`.
#[cfg(target_arch = "wasm32")]
pub fn inc_performance_counter() -> u64 {
    performance_counter()
}

/// Increments a mock performance counter and panics whenever the instruction limit is exceeded.
#[cfg(not(target_arch = "wasm32"))]
pub fn inc_performance_counter() -> u64 {
    PERFORMANCE_COUNTER.with(|pc| {
        *pc.borrow_mut() += PERFORMANCE_COUNTER_STEP.with(|ps| *ps.borrow());

        if *pc.borrow() > INSTRUCTIONS_LIMIT {
            panic!("instructions limit exceeded");
        }
    });

    performance_counter()
}

/// Returns the current instruction count.
#[cfg(target_arch = "wasm32")]
pub fn performance_counter() -> u64 {
    ic_cdk::api::performance_counter(0)
}

/// Returns the current instruction count.
#[cfg(not(target_arch = "wasm32"))]
pub fn performance_counter() -> u64 {
    PERFORMANCE_COUNTER.with(|pc| *pc.borrow())
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

#[cfg(target_arch = "wasm32")]
pub fn msg_cycles_available() -> u64 {
    ic_cdk::api::call::msg_cycles_available()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn msg_cycles_available() -> u64 {
    u64::MAX
}

#[cfg(target_arch = "wasm32")]
pub fn msg_cycles_accept(max_amount: u64) -> u64 {
    ic_cdk::api::call::msg_cycles_accept(max_amount)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn msg_cycles_accept(max_amount: u64) -> u64 {
    CYCLES_BALANCE.with(|c| *c.borrow_mut() += max_amount);
    max_amount
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
pub fn get_cycles_balance() -> u64 {
    CYCLES_BALANCE.with(|c| *c.borrow())
}
