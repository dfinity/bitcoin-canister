use ic_btc_interface::{Config, Flag, SetConfigRequest};
use ic_cdk_macros::{init, post_upgrade, query, update};
use std::cell::RefCell;

thread_local! {
    static API_ACCESS: RefCell<Flag> = RefCell::new(Flag::Enabled);
}

#[init]
fn init() {}

#[post_upgrade]
fn post_upgrade() {
    init()
}

#[query]
fn get_config() -> Config {
    Config {
        api_access: API_ACCESS.with(|cell| *cell.borrow()),
        ..Default::default()
    }
}

#[update]
async fn set_config(request: SetConfigRequest) {
    if let Some(flag) = request.api_access {
        API_ACCESS.with(|cell| *cell.borrow_mut() = flag);
    }
}

fn main() {}
