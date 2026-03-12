use crate::config::{Canister, Config};
use crate::fetch::BlockInfo;
use crate::{
    CanisterCallErrors, API_ACCESS_TARGET, BLOCK_INFO_DATA, CANISTER_CALL_ERRORS, CANISTER_HEIGHT,
};
use ic_btc_interface::Flag;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{Cell, DefaultMemoryImpl};
use std::cell::RefCell;

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static CONFIG: RefCell<Cell<Config, Memory>> = RefCell::new(Cell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))), Config::default()));
}

/// Initializes both the target canister and its default config.
pub fn set_canister_config(canister: Canister) {
    set_config(Config::for_target(canister));
}

/// Returns the configuration from the stable storage.
pub fn get_config() -> Config {
    CONFIG.with(|cell| cell.borrow().get().clone())
}

/// Sets the configuration in the stable storage.
pub fn set_config(config: Config) {
    CONFIG.with(|cell| cell.borrow_mut().set(config));
}

/// Sets the monitored canister's main chain height.
pub fn set_canister_height(height: Option<u64>) {
    CANISTER_HEIGHT.with(|cell| *cell.borrow_mut() = height);
}

/// Returns the monitored canister's main chain height.
pub fn get_canister_height() -> Option<u64> {
    CANISTER_HEIGHT.with(|cell| *cell.borrow())
}

/// Inserts explorer data into the local storage.
pub fn insert_block_info(info: BlockInfo) {
    BLOCK_INFO_DATA.with(|cell| {
        cell.borrow_mut().insert(info.provider.to_string(), info);
    });
}

/// Returns block info for an API provider.
pub fn get_block_info(provider: &str) -> Option<BlockInfo> {
    BLOCK_INFO_DATA.with(|cell| cell.borrow().get(provider).cloned())
}

/// Sets the API access into the local storage.
pub fn set_api_access_target(flag: Option<Flag>) {
    API_ACCESS_TARGET.with(|cell| *cell.borrow_mut() = flag);
}

/// Returns the API access from the local storage.
pub fn get_api_access_target() -> Option<Flag> {
    API_ACCESS_TARGET.with(|cell| *cell.borrow())
}

/// Increments the error counter for a `get_blockchain_info` call.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn inc_get_blockchain_info_errors() {
    CANISTER_CALL_ERRORS.with(|cell| cell.borrow_mut().get_blockchain_info += 1);
}

/// Increments the error counter for a `get_config` call.
pub fn inc_get_config_errors() {
    CANISTER_CALL_ERRORS.with(|cell| cell.borrow_mut().get_config += 1);
}

/// Increments the error counter for a `set_config` call.
pub fn inc_set_config_errors() {
    CANISTER_CALL_ERRORS.with(|cell| cell.borrow_mut().set_config += 1);
}

/// Returns the current canister call error counts.
pub fn get_canister_call_errors() -> CanisterCallErrors {
    CANISTER_CALL_ERRORS.with(|cell| cell.borrow().clone())
}
