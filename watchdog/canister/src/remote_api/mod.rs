mod http;
mod storage;

mod api_bitaps_com;
pub use api_bitaps_com::ApiBitapsCom;

mod api_blockcypher_com;
pub use api_blockcypher_com::ApiBlockcypherCom;

mod bitcoin_canister;
pub use bitcoin_canister::BitcoinCanister;

mod blockchain_info;
pub use blockchain_info::BlockchainInfo;

mod blockstream_info;
pub use blockstream_info::BlockstreamInfo;

mod chain_api_btc_com;
pub use chain_api_btc_com::ChainApiBtcCom;

#[derive(Eq, PartialEq, Debug)]
pub enum RemoteAPI {
    ApiBitapsCom,
    ApiBlockcypherCom,
    BitcoinCanister,
    BlockchainInfo,
    BlockstreamInfo,
    ChainApiBtcCom,
}

impl RemoteAPI {
    fn host(&self) -> &'static str {
        match self {
            RemoteAPI::ApiBitapsCom => ApiBitapsCom::host(),
            RemoteAPI::ApiBlockcypherCom => ApiBlockcypherCom::host(),
            RemoteAPI::BitcoinCanister => BitcoinCanister::host(),
            RemoteAPI::BlockchainInfo => BlockchainInfo::host(),
            RemoteAPI::BlockstreamInfo => BlockstreamInfo::host(),
            RemoteAPI::ChainApiBtcCom => ChainApiBtcCom::host(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_to_string() {
        use RemoteAPI::*;

        let test_cases = [
            (ApiBitapsCom, "api.bitaps.com"),
            (ApiBlockcypherCom, "api.blockcypher.com"),
            (BitcoinCanister, "ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app"),
            (BlockchainInfo, "blockchain.info"),
            (BlockstreamInfo, "blockstream.info"),
            (ChainApiBtcCom, "chain.api.btc.com"),
        ];
        for (variant, host) in test_cases {
            assert_eq!(variant.host(), host);
        }
    }
}
