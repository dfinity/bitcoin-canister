use crate::block_apis::CandidBlockApi;
use crate::config::Config;
use crate::{fetch::BlockInfo, API_ACCESS_TARGET, BLOCK_INFO_DATA};
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
pub fn insert_block_info(info: BlockInfo) {
    BLOCK_INFO_DATA.with(|cell| {
        cell.borrow_mut().insert(info.provider.clone(), info);
    });
}

/// Returns the data from the local storage.
pub fn get_block_info(provider: &CandidBlockApi) -> Option<BlockInfo> {
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
