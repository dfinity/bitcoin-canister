use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::config::Config;
use crate::fetch::BlockInfo;
use crate::API_ACCESS_TARGET;
use crate::BLOCK_INFO_DATA;
use crate::CONFIG;
use ic_btc_interface::Flag;

/// Returns the configuration from the local storage.
pub fn get_config() -> Config {
    CONFIG.with(|cell| cell.borrow().clone())
}

/// Sets the configuration in the local storage.
#[cfg(test)]
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
pub fn get_block_info(provider: &BitcoinBlockApi) -> Option<BlockInfo> {
    BLOCK_INFO_DATA.with(|cell| cell.borrow().get(provider).cloned())
}

/// Sets the API access into the local storage.
pub fn set_api_access_target(flag: Option<Flag>) {
    API_ACCESS_TARGET.with(|cell| *cell.borrow_mut() = flag);
}

/// Returns the API access from the local storage.
pub fn get_api_access_target() -> Option<Flag> {
    API_ACCESS_TARGET.with(|cell| cell.borrow().clone())
}
