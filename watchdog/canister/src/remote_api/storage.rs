use crate::info::Config;
use crate::time;
use crate::types::BlockHeight;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Copy, Clone)]
struct Entry {
    insertion_time: time::CanisterUptime,

    height: BlockHeight,
}

// This is a thread-local storage for the remote API data.
thread_local! {
    static STORAGE: RefCell<HashMap<String, Entry>> = RefCell::default();
}

/// Inserts a new entry into the storage.
pub(crate) fn insert(key: &str, height: BlockHeight) {
    let entry = Entry {
        insertion_time: time::now(),
        height,
    };
    STORAGE.with(|cell| cell.borrow_mut().insert(key.to_string(), entry));
}

/// Returns the entry from the storage.
pub(crate) fn get(key: &str) -> Option<BlockHeight> {
    let entry = STORAGE.with(|cell| cell.borrow().get(&key.to_string()).copied());
    match entry {
        None => None, // No data found.
        Some(e) => {
            let elapsed = time::now() - e.insertion_time;
            if elapsed > Duration::from_millis(Config::default().storage_ttl_millis) {
                None // Drop stale data.
            } else {
                Some(e.height)
            }
        }
    }
}
