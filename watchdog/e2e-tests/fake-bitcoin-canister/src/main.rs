use ic_btc_interface::Config;
use ic_cdk_macros::{init, post_upgrade, query};

#[init]
fn init() {}

#[post_upgrade]
fn post_upgrade() {
    init()
}

#[query]
fn get_config() -> Config {
    Config::default()
}

fn main() {}
