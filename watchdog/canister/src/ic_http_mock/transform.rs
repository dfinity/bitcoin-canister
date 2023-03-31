use candid::Principal;
use ic_cdk::api::management_canister::http_request::{
    HttpResponse, TransformArgs, TransformContext, TransformFunc,
};
use std::cell::RefCell;
use std::collections::HashMap;

pub type TransformFn = fn(TransformArgs) -> HttpResponse;

thread_local! {
    static TRANSFORM_FUNCTIONS: RefCell<HashMap<String, TransformFn>> = RefCell::default();
}

fn insert(function_name: String, func: TransformFn) {
    TRANSFORM_FUNCTIONS.with(|cell| cell.borrow_mut().insert(function_name, func));
}

pub(crate) fn get(function_name: String) -> Option<TransformFn> {
    TRANSFORM_FUNCTIONS.with(|cell| cell.borrow().get(&function_name).copied())
}

pub struct TransformContextBuilder {
    func: TransformFn,
    context: Vec<u8>,
}

fn get_function_name<F>(_: F) -> &'static str {
    let full_name = std::any::type_name::<F>();
    match full_name.rfind(':') {
        Some(index) => &full_name[index + 1..],
        None => full_name,
    }
}

fn transform_no_op(args: TransformArgs) -> HttpResponse {
    args.response
}

impl TransformContextBuilder {
    pub fn new() -> Self {
        Self {
            func: transform_no_op,
            context: vec![],
        }
    }

    pub fn func(mut self, func: TransformFn) -> Self {
        self.func = func;
        self
    }

    pub fn context(mut self, context: Vec<u8>) -> Self {
        self.context = context;
        self
    }

    pub fn build(self) -> TransformContext {
        let function_name = get_function_name(self.func).to_string();
        insert(function_name.clone(), self.func);

        TransformContext {
            function: TransformFunc(candid::Func {
                principal: Principal::anonymous(),
                method: function_name,
            }),
            context: self.context,
        }
    }
}
