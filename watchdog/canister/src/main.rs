use watchdog::tick_async;
use watchdog::{Config, Info};

#[ic_cdk_macros::init]
fn init() {
    let config = Config::default();
    let interval = std::time::Duration::from_secs(config.timer_interval_secs as u64);
    watchdog::print(&format!(
        "Starting a periodic task with interval {interval:?}"
    ));
    ic_cdk_timers::set_timer_interval(interval, || ic_cdk::spawn(tick_async()));
}

#[ic_cdk_macros::post_upgrade]
fn post_upgrade() {
    init()
}

#[ic_cdk_macros::query]
pub fn get_info() -> Info {
    watchdog::get_info()
}

#[ic_cdk_macros::query]
pub fn get_info_json() -> String {
    watchdog::get_info().as_json_str()
}

fn main() {}
