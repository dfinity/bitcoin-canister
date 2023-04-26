use crate::endpoints::*;

/// Mocks all the mainnet outcalls to be successful.
pub fn mock_mainnet_outcalls() {
    let mocks = [
        (
            endpoint_api_blockchair_com_block_mainnet(),
            API_BLOCKCHAIR_COM_MAINNET_RESPONSE,
        ),
        (
            endpoint_api_blockcypher_com_block_mainnet(),
            API_BLOCKCYPHER_COM_MAINNET_RESPONSE,
        ),
        (
            endpoint_bitcoin_canister(),
            BITCOIN_CANISTER_MAINNET_RESPONSE,
        ),
        (
            endpoint_blockchain_info_hash_mainnet(),
            BLOCKCHAIN_INFO_HASH_MAINNET_RESPONSE,
        ),
        (
            endpoint_blockchain_info_height_mainnet(),
            BLOCKCHAIN_INFO_HEIGHT_MAINNET_RESPONSE,
        ),
        (
            endpoint_blockstream_info_hash_mainnet(),
            BLOCKSTREAM_INFO_HASH_MAINNET_RESPONSE,
        ),
        (
            endpoint_blockstream_info_height_mainnet(),
            BLOCKSTREAM_INFO_HEIGHT_MAINNET_RESPONSE,
        ),
        (
            endpoint_chain_api_btc_com_block_mainnet(),
            CHAIN_API_BTC_COM_MAINNET_RESPONSE,
        ),
    ];
    for (config, response_body) in mocks {
        let request = config.request();
        let mock_response = ic_http::create_response()
            .status(200)
            .body(response_body)
            .build();
        ic_http::mock::mock(request, mock_response);
    }
}

/// Mocks all the testnet outcalls to be successful.
pub fn mock_testnet_outcalls() {
    let mocks = [
        (
            endpoint_api_blockchair_com_block_testnet(),
            API_BLOCKCHAIR_COM_TESTNET_RESPONSE,
        ),
        (
            endpoint_api_blockcypher_com_block_testnet(),
            API_BLOCKCYPHER_COM_TESTNET_RESPONSE,
        ),
        (
            endpoint_bitcoin_canister(),
            BITCOIN_CANISTER_TESTNET_RESPONSE,
        ),
        (
            endpoint_blockstream_info_hash_testnet(),
            BLOCKSTREAM_INFO_HASH_TESTNET_RESPONSE,
        ),
        (
            endpoint_blockstream_info_height_testnet(),
            BLOCKSTREAM_INFO_HEIGHT_TESTNET_RESPONSE,
        ),
    ];
    for (config, response_body) in mocks {
        let request = config.request();
        let mock_response = ic_http::create_response()
            .status(200)
            .body(response_body)
            .build();
        ic_http::mock::mock(request, mock_response);
    }
}

/// Mocks all the outcalls to fail with status code 404.
pub fn mock_all_outcalls_404() {
    let mocks = [
        endpoint_api_blockchair_com_block_mainnet(),
        endpoint_api_blockchair_com_block_testnet(),
        endpoint_api_blockcypher_com_block_mainnet(),
        endpoint_api_blockcypher_com_block_testnet(),
        endpoint_bitcoin_canister(),
        endpoint_blockchain_info_hash_mainnet(),
        endpoint_blockchain_info_height_mainnet(),
        endpoint_blockstream_info_hash_mainnet(),
        endpoint_blockstream_info_hash_testnet(),
        endpoint_blockstream_info_height_mainnet(),
        endpoint_blockstream_info_height_testnet(),
        endpoint_chain_api_btc_com_block_mainnet(),
    ];
    for config in mocks {
        let request = config.request();
        let mock_response = ic_http::create_response().status(404).build();
        ic_http::mock::mock(request, mock_response);
    }
}

/// Mocks all the outcalls to abuse the API.
pub fn mock_all_outcalls_abusing_api() {
    let mocks = [
        endpoint_api_blockchair_com_block_mainnet(),
        endpoint_api_blockchair_com_block_testnet(),
        endpoint_api_blockcypher_com_block_mainnet(),
        endpoint_api_blockcypher_com_block_testnet(),
        endpoint_bitcoin_canister(),
        endpoint_blockchain_info_hash_mainnet(),
        endpoint_blockchain_info_height_mainnet(),
        endpoint_blockstream_info_hash_mainnet(),
        endpoint_blockstream_info_hash_testnet(),
        endpoint_blockstream_info_height_mainnet(),
        endpoint_blockstream_info_height_testnet(),
        endpoint_chain_api_btc_com_block_mainnet(),
    ];
    for config in mocks {
        let request = config.request();
        let mock_response = ic_http::create_response()
            .status(200)
            .body(DONT_ABUSE_THE_API)
            .build();
        ic_http::mock::mock(request, mock_response);
    }
}

pub const DONT_ABUSE_THE_API: &str = r#"Don't abuse the API. Please contact support."#;

// https://api.blockchair.com/bitcoin/stats
pub const API_BLOCKCHAIR_COM_MAINNET_RESPONSE: &str = r#"{
    "data":
    {
        "blocks":783771,
        "transactions":820266066,
        "outputs":2309684029,
        "circulation":1933603979497096,
        "blocks_24h":148,
        "transactions_24h":370690,
        "difficulty":46843400286277,
        "volume_24h":97687710547510,
        "mempool_transactions":29979,
        "mempool_size":203718813,
        "mempool_tps":4.433333333333334,
        "mempool_total_fee_usd":52388.2163,
        "best_block_height":700002,
        "best_block_hash":"0000000000000000000aaa222222222222222222222222222222222222222222",
        "best_block_time":"2023-04-03 14:04:50",
        "blockchain_size":470319339145,
        "average_transaction_fee_24h":6780,
        "inflation_24h":92500000000,
        "median_transaction_fee_24h":3495,
        "cdd_24h":5327187.228927112,
        "mempool_outputs":637712,
        "largest_transaction_24h":
        {
            "hash":"0fde94d2ca0eb734f83c166626bf22dea861deb6aba69e7d1c28f1171a922f13",
            "value_usd":427008416
        },
        "nodes":7718,
        "hashrate_24h":"345095835785586196564",
        "inflation_usd_24h":26150675,
        "average_transaction_fee_usd_24h":1.9170246384876852,
        "median_transaction_fee_usd_24h":0.9880714500000001,
        "market_price_usd":28271,
        "market_price_btc":1,
        "market_price_usd_change_24h_percentage":-0.15658,
        "market_cap_usd":546793120160,
        "market_dominance_percentage":44.66,
        "next_retarget_time_estimate":"2023-04-06 16:32:29",
        "next_difficulty_estimate":44336619371627,
        "countdowns":[],
        "suggested_transaction_fee_per_byte_sat":21,
        "hodling_addresses":45818990
    }    
}"#;

// https://api.blockchair.com/bitcoin/testnet/stats
pub const API_BLOCKCHAIR_COM_TESTNET_RESPONSE: &str = r#"{
    "data":
    {
        "blocks":2431136,
        "transactions":65448595,
        "outputs":173565382,
        "circulation":2099216984092285,
        "blocks_24h":96,
        "transactions_24h":9427,
        "difficulty":1,
        "volume_24h":5268257522789,
        "mempool_transactions":112,
        "mempool_size":39045,
        "mempool_tps":0.06666666666666667,
        "mempool_total_fee_usd":0,
        "best_block_height":2000001,
        "best_block_hash":"0000000000000000000fff222222222222222222222222222222222222222222",
        "best_block_time":"2023-04-26 17:12:02",
        "blockchain_size":28774784525,
        "average_transaction_fee_24h":4780,
        "inflation_24h":234374976,
        "median_transaction_fee_24h":247,
        "cdd_24h":132168.45577533677,
        "mempool_outputs":453,
        "largest_transaction_24h":
        {
            "hash":"6b50cb5842a049a8aa148f40acad0d20970e5100ed7659938f5f0f95ca2c5d4f",
            "value_usd":0
        },
        "hashrate_24h":"4772185",
        "inflation_usd_24h":0,
        "average_transaction_fee_usd_24h":0,
        "median_transaction_fee_usd_24h":0,
        "market_price_usd":0,
        "market_price_btc":0,
        "market_price_usd_change_24h_percentage":0,
        "market_cap_usd":0,
        "market_dominance_percentage":0,
        "next_retarget_time_estimate":"2023-04-27 12:46:52",
        "next_difficulty_estimate":139532120,
        "suggested_transaction_fee_per_byte_sat":1,
        "hodling_addresses":10038668
    },
    "context":
    {
        "code":200,
        "source":"A",
        "state":2431135,
        "market_price_usd":29794,
        "cache":
        {
            "live":false,
            "duration":"Ignore",
            "since":"2023-04-26 17:24:41",
            "until":"2023-04-26 17:25:52",
            "time":1.9073486328125e-6
        },
        "api":
        {
            "version":"2.0.95-ie",
            "last_major_update":"2022-11-07 02:00:00",
            "next_major_update":null,
            "documentation":"https:\/\/blockchair.com\/api\/docs",
            "notice":"Please note that on November 7th, 2022 public support for the following blockchains was dropped: EOS, Bitcoin SV"
        },
        "servers":"API4,TBTC0",
        "time":1.7573418617248535,
        "render_time":0.0011322498321533203,
        "full_time":0.0011341571807861328,
        "request_cost":1
    }
}"#;

// https://api.blockcypher.com/v1/btc/main
pub const API_BLOCKCYPHER_COM_MAINNET_RESPONSE: &str = r#"{
    "name": "BTC.main",
    "height": 700003,
    "hash": "0000000000000000000aaa333333333333333333333333333333333333333333",
    "time": "2023-03-25T08:38:41.081949161Z",
    "latest_url": "https://api.blockcypher.com/v1/btc/main/blocks/00000000000000000004f7e4f909f1e9ebbe3db9c94e5165cdda946f8a6a4e72",
    "previous_hash": "0000000000000000000aaa222222222222222222222222222222222222222222",
    "previous_url": "https://api.blockcypher.com/v1/btc/main/blocks/00000000000000000001a4e2dc423c9d167fa6ffd9f34bf0c6d919521ef82003",
    "peer_count": 243,
    "unconfirmed_count": 7543,
    "high_fee_per_kb": 33350,
    "medium_fee_per_kb": 19047,
    "low_fee_per_kb": 12258,
    "last_fork_height": 781277,
    "last_fork_hash": "0000000000000000000388f42000fa901c01f2bfae36042bbae133ee430e6485"
}"#;

// https://api.blockcypher.com/v1/btc/test3
pub const API_BLOCKCYPHER_COM_TESTNET_RESPONSE: &str = r#"{
    "name": "BTC.test3",
    "height": 2000002,
    "hash": "0000000000000000000fff333333333333333333333333333333333333333333",
    "time": "2023-04-26T17:12:11.044585287Z",
    "latest_url": "https://api.blockcypher.com/v1/btc/test3/blocks/0000000000008d9497a398933d6618c6a39a6c818c22e82ef864f0a53c7bc4c1",
    "previous_hash": "0000000000000000000fff222222222222222222222222222222222222222222",
    "previous_url": "https://api.blockcypher.com/v1/btc/test3/blocks/00000000000000150d0869032cacc4af7b72a70a60e6d41805543a471e17050e",
    "peer_count": 284,
    "unconfirmed_count": 62,
    "high_fee_per_kb": 44424,
    "medium_fee_per_kb": 29773,
    "low_fee_per_kb": 16199,
    "last_fork_height": 2428426,
    "last_fork_hash": "00000000000011f558264dc907379ec11e62420f6224f0b081dc6155e9a6e239"
}"#;

// https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics
pub const BITCOIN_CANISTER_MAINNET_RESPONSE: &str = r#"{
    # HELP main_chain_height Height of the main chain.
    # TYPE main_chain_height gauge
    main_chain_height 700007 1680014894644
    # HELP stable_height The height of the latest stable block.
    # TYPE stable_height gauge
    stable_height 782801 1680014894644
    # HELP utxos_length The number of UTXOs in the set.
    # TYPE utxos_length gauge
    utxos_length 86798838 1680014894644
    # HELP address_utxos_length The number of UTXOs that are owned by supported addresses.
    # TYPE address_utxos_length gauge
    address_utxos_length 86294218 1680014894644
}"#;

// https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics
pub const BITCOIN_CANISTER_TESTNET_RESPONSE: &str = r#"{
    # HELP main_chain_height Height of the main chain.
    # TYPE main_chain_height gauge
    main_chain_height 2000007 1682533330541
    # HELP stable_height The height of the latest stable block.
    # TYPE stable_height gauge
    stable_height 2430866 1682533330541
    # HELP utxos_length The number of UTXOs in the set.
    # TYPE utxos_length gauge
    utxos_length 28755498 1682533330541
    # HELP address_utxos_length The number of UTXOs that are owned by supported addresses.
    # TYPE address_utxos_length gauge
    address_utxos_length 28388537 1682533330541
}"#;

// https://blockchain.info/q/latesthash
pub const BLOCKCHAIN_INFO_HASH_MAINNET_RESPONSE: &str =
    r#"0000000000000000000aaa444444444444444444444444444444444444444444"#;

// https://blockchain.info/q/getblockcount
pub const BLOCKCHAIN_INFO_HEIGHT_MAINNET_RESPONSE: &str = r#"700004"#;

// https://blockstream.info/api/blocks/tip/hash
pub const BLOCKSTREAM_INFO_HASH_MAINNET_RESPONSE: &str =
    r#"0000000000000000000aaa555555555555555555555555555555555555555555"#;

// https://blockstream.info/api/blocks/tip/height
pub const BLOCKSTREAM_INFO_HEIGHT_MAINNET_RESPONSE: &str = r#"700005"#;

// https://blockstream.info/testnet/api/blocks/tip/hash
pub const BLOCKSTREAM_INFO_HASH_TESTNET_RESPONSE: &str =
    r#"0000000000000000000fff555555555555555555555555555555555555555555"#;

// https://blockstream.info/testnet/api/blocks/tip/height
pub const BLOCKSTREAM_INFO_HEIGHT_TESTNET_RESPONSE: &str = r#"2000003"#;

// https://chain.api.btc.com/v3/block/latest
pub const CHAIN_API_BTC_COM_MAINNET_RESPONSE: &str = r#"{
    "data": {
        "height":700006,
        "version":538968064,
        "mrkl_root":"fd7a75292e02050465de1ff8a98ea7e0dbe22f6107a3ee89c9de40e32166ad23",
        "timestamp":1679733439,
        "bits":386269758,
        "nonce":110254631,
        "hash":"0000000000000000000aaa666666666666666666666666666666666666666666",
        "prev_block_hash":"0000000000000000000aaa555555555555555555555555555555555555555555",
        "next_block_hash":"0000000000000000000000000000000000000000000000000000000000000000",
        "size":1561960,
        "pool_difficulty":56653058926588,
        "difficulty":46843400286276,
        "difficulty_double":46843400286276.55,
        "tx_count":2957,
        "reward_block":625000000,
        "reward_fees":32773177,
        "confirmations":1,
        "is_orphan":false,
        "curr_max_timestamp":1679733439,
        "is_sw_block":true,
        "stripped_size":810332,
        "sigops":14267,
        "weight":3992956,
        "extras": {
            "pool_name":"PEGA Pool",
            "pool_link":"https://www.pega-pool.com"
        }
    },
    "err_code":0,
    "err_no":0,
    "message":"success",
    "status":"success"
}"#;
