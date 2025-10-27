use crate::bitcoin_block_apis::CandidBlockApi;
use crate::{config::Config, fetch::BlockInfo, API_ACCESS_TARGET, BLOCK_INFO_DATA, CONFIG};
use ic_btc_interface::Flag;

/// Returns the configuration from the local storage.
pub fn get_config() -> Config {
    CONFIG.with(|cell| cell.borrow().clone())
}

/// Sets the configuration in the local storage.
pub fn set_config(config: Config) {
    CONFIG.with(|cell| *cell.borrow_mut() = config);
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
