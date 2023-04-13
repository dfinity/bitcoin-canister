use crate::bitcoin_block_apis::BitcoinBlockApi;
use crate::fetch::BlockInfo;
use crate::BLOCK_INFO_DATA;

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
