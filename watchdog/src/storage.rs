use crate::block_apis::BlockApi;
use crate::config::Config;
use crate::fetch::BlockInfoInternal;
use crate::{API_ACCESS_TARGET, BLOCK_INFO_DATA, HEALTH_STATUS_CALLS};
use ic_btc_interface::Flag;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{Cell, DefaultMemoryImpl};
use std::cell::RefCell;

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static CONFIG: RefCell<Cell<Config, Memory>> = RefCell::new(Cell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), Config::default()));
}

/// Returns the configuration from the stable storage.
pub fn get_config() -> Config {
    CONFIG.with(|cell| cell.borrow().get().clone())
}

/// Sets the configuration in the stable storage.
pub fn set_config(config: Config) {
    CONFIG.with(|cell| cell.borrow_mut().set(config));
}

/// Inserts the data into the local storage.
pub fn insert_block_info(info: BlockInfoInternal) {
    BLOCK_INFO_DATA.with(|cell| {
        cell.borrow_mut().insert(info.provider.clone(), info);
    });
}

/// Returns the data from the local storage.
pub fn get_block_info(provider: &BlockApi) -> Option<BlockInfoInternal> {
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

/// Increments the health_status endpoint call counter.
pub fn increment_health_status_calls() {
    HEALTH_STATUS_CALLS.with(|cell| {
        *cell.borrow_mut() += 1;
    });
}

/// Returns the number of health_status endpoint calls.
pub fn get_health_status_calls() -> u64 {
    HEALTH_STATUS_CALLS.with(|cell| *cell.borrow())
}
