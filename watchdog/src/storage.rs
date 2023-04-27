use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::config::Config;
use crate::fetch::BlockInfo;
use crate::BLOCK_INFO_DATA;
use crate::CONFIG;

/// Inserts the data into the local storage.
pub fn insert(info: BlockInfo) {
    BLOCK_INFO_DATA.with(|data| {
        data.write().unwrap().insert(info.provider.clone(), info);
    });
}

/// Returns the data from the local storage.
pub fn get(provider: &BitcoinBlockApi) -> Option<BlockInfo> {
    BLOCK_INFO_DATA.with(|data| data.read().unwrap().get(provider).cloned())
}

/// Returns the configuration from the local storage.
pub fn get_config() -> Config {
    CONFIG.with(|cell: &std::sync::RwLock<Config>| cell.read().unwrap().clone())
}

/// Sets the configuration in the local storage.
#[cfg(test)]
pub fn set_config(config: Config) {
    CONFIG.with(|cell| *cell.write().unwrap() = config);
}
