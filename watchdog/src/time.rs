use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::cell::RefCell;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    /// Canister inner uptime since start in nanoseconds.
    static START: RefCell<Instant> = RefCell::new(Instant::now());
}

/// Canister inner uptime since start in nanoseconds.
#[derive(Copy, Clone)]
pub struct CanisterUptime(u128);

impl CanisterUptime {
    fn from_nanos(nanos: u128) -> Self {
        CanisterUptime(nanos)
    }
}

impl std::ops::Sub<CanisterUptime> for CanisterUptime {
    type Output = Duration;

    fn sub(self, other: CanisterUptime) -> Duration {
        let lhs = self.0;
        let rhs = other.0;
        let sub = lhs - rhs;
        Duration::from_nanos(sub as u64)
    }
}

/// Canister inner uptime since start in nanoseconds.
#[cfg(not(target_arch = "wasm32"))]
pub fn now() -> CanisterUptime {
    let nanos = START.with(|cell| cell.borrow().elapsed().as_nanos());
    CanisterUptime::from_nanos(nanos)
}

/// Canister inner uptime since start in nanoseconds.
#[cfg(target_arch = "wasm32")]
pub fn now() -> CanisterUptime {
    // Current timestamp, in nanoseconds since the epoch (1970-01-01).
    let nanos = ic_cdk::api::time();
    CanisterUptime::from_nanos(nanos.into())
}
