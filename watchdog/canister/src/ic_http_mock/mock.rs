use ic_cdk::api::management_canister::http_request::{CanisterHttpRequestArgument, HttpResponse};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct MockEntry {
    pub(crate) request: CanisterHttpRequestArgument,
    pub(crate) response: HttpResponse,
    pub(crate) delay: Option<Duration>,
}

thread_local! {
    static MOCK_RESPONSES: RefCell<HashMap<String, MockEntry>> = RefCell::default();
}

fn insert(request: CanisterHttpRequestArgument, response: HttpResponse, delay: Option<Duration>) {
    MOCK_RESPONSES.with(|cell| {
        cell.borrow_mut().insert(
            hash(&request),
            MockEntry {
                request,
                response,
                delay,
            },
        );
    });
}

pub(crate) fn get(request: &CanisterHttpRequestArgument) -> Option<MockEntry> {
    MOCK_RESPONSES.with(|cell| cell.borrow().get(&hash(&request)).cloned())
}

fn hash(request: &CanisterHttpRequestArgument) -> String {
    let mut hash = String::new();

    hash.push_str(&request.url);
    hash.push_str(&format!("{:?}", request.max_response_bytes));
    hash.push_str(&format!("{:?}", request.method));
    for header in request.headers.iter() {
        hash.push_str(&header.name);
        hash.push_str(&header.value);
    }
    let body = String::from_utf8(request.body.as_ref().unwrap_or(&vec![]).clone())
        .expect("Raw response is not UTF-8 encoded.");
    hash.push_str(&body);
    //hash.push_str(request.transform.map());

    hash
}

pub fn mock(request: &CanisterHttpRequestArgument, response: &HttpResponse) {
    insert(request.clone(), response.clone(), None);
}

pub fn mock_with_delay(
    request: &CanisterHttpRequestArgument,
    response: &HttpResponse,
    delay: Duration,
) {
    insert(request.clone(), response.clone(), Some(delay));
}
