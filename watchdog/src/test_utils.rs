use crate::endpoints::*;
use crate::http::HttpRequestConfig;

/// Mocks all the Bitcoin mainnet outcalls to be successful.
pub fn mock_bitcoin_mainnet_outcalls() {
    let mocks = [
        (
            endpoint_bitcoin_mainnet_api_bitaps_com(),
            BITCOIN_MAINNET_API_BITAPS_COM_RESPONSE,
        ),
        (
            endpoint_bitcoin_mainnet_api_blockchair_com(),
            BITCOIN_MAINNET_API_BLOCKCHAIR_COM_RESPONSE,
        ),
        (
            endpoint_bitcoin_mainnet_api_blockcypher_com(),
            BITCOIN_MAINNET_API_BLOCKCYPHER_COM_RESPONSE,
        ),
        (
            endpoint_bitcoin_canister(),
            BITCOIN_MAINNET_CANISTER_RESPONSE,
        ),
        (
            endpoint_bitcoin_mainnet_blockchain_info(),
            BITCOIN_MAINNET_BLOCKCHAIN_INFO_RESPONSE,
        ),
        (
            endpoint_bitcoin_mainnet_blockstream_info(),
            BITCOIN_MAINNET_BLOCKSTREAM_INFO_RESPONSE,
        ),
        (
            endpoint_bitcoin_mainnet_mempool(),
            BITCOIN_MAINNET_MEMPOOL_RESPONSE,
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

/// Mocks all the Bitcoin testnet outcalls to be successful.
pub fn mock_bitcoin_testnet_outcalls() {
    let mocks = [
        (
            endpoint_bitcoin_canister(),
            BITCOIN_TESTNET_CANISTER_RESPONSE,
        ),
        (
            endpoint_bitcoin_testnet_mempool(),
            BITCOIN_TESTNET_MEMPOOL_RESPONSE,
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

/// Mocks all the Dogecoin mainnet outcalls to be successful.
pub fn mock_dogecoin_mainnet_outcalls() {
    let mocks = [
        (
            endpoint_dogecoin_mainnet_api_blockchair_com(),
            DOGECOIN_MAINNET_API_BLOCKCHAIR_COM_RESPONSE,
        ),
        (
            endpoint_dogecoin_mainnet_api_blockcypher_com(),
            DOGECOIN_MAINNET_API_BLOCKCYPHER_COM_RESPONSE,
        ),
        (
            endpoint_dogecoin_canister(),
            DOGECOIN_MAINNET_CANISTER_RESPONSE,
        ),
        (
            endpoint_dogecoin_mainnet_psy_protocol(),
            DOGECOIN_MAINNET_PSY_PROTOCOL_RESPONSE,
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

fn all_mock_outcalls() -> Vec<HttpRequestConfig> {
    vec![
        endpoint_bitcoin_mainnet_api_blockchair_com(),
        endpoint_bitcoin_mainnet_api_blockcypher_com(),
        endpoint_bitcoin_canister(),
        endpoint_bitcoin_mainnet_blockchain_info(),
        endpoint_bitcoin_mainnet_blockstream_info(),
        endpoint_dogecoin_mainnet_api_blockchair_com(),
        endpoint_dogecoin_mainnet_api_blockcypher_com(),
        endpoint_dogecoin_canister(),
        endpoint_dogecoin_mainnet_psy_protocol(),
        endpoint_bitcoin_mainnet_mempool(),
        endpoint_bitcoin_testnet_mempool(),
    ]
}

/// Mocks all the outcalls to fail with status code 404.
pub fn mock_all_outcalls_404() {
    for config in all_mock_outcalls() {
        let request = config.request();
        let mock_response = ic_http::create_response().status(404).build();
        ic_http::mock::mock(request, mock_response);
    }
}

/// Mocks all the outcalls to abuse the API.
pub fn mock_all_outcalls_abusing_api() {
    for config in all_mock_outcalls() {
        let request = config.request();
        let mock_response = ic_http::create_response()
            .status(200)
            .body(DONT_ABUSE_THE_API)
            .build();
        ic_http::mock::mock(request, mock_response);
    }
}

pub const DONT_ABUSE_THE_API: &str = r#"Don't abuse the API. Please contact support."#;

// https://api.bitaps.com/btc/v1/blockchain/block/last
pub const BITCOIN_MAINNET_API_BITAPS_COM_RESPONSE: &str = r#"{
    "data": {
        "height": 700001,
        "hash": "0000000000000000000aaa111111111111111111111111111111111111111111",
        "header": "AGAAILqkI+SFlsu4FRCwVNiwU3Eku+N/g9sEAAAAAAAAAAAAH1tWFGtObfxfaOeXVwH9txRFHWS4V+N24n9AyliR1S4Yvghko4kGFwdzNef9XA4=",
        "adjustedTimestamp": 1678294552
    },
    "time": 0.0018
}"#;

// https://api.blockchair.com/bitcoin/stats
pub const BITCOIN_MAINNET_API_BLOCKCHAIR_COM_RESPONSE: &str = r#"{
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

// https://api.blockcypher.com/v1/btc/main
pub const BITCOIN_MAINNET_API_BLOCKCYPHER_COM_RESPONSE: &str = r#"{
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

// https://blockchain.info/q/getblockcount
pub const BITCOIN_MAINNET_BLOCKCHAIN_INFO_RESPONSE: &str = r#"700004"#;

// https://blockstream.info/api/blocks/tip/height
pub const BITCOIN_MAINNET_BLOCKSTREAM_INFO_RESPONSE: &str = r#"700005"#;

// https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics
// https://axowo-ciaaa-aaaad-acs7q-cai.raw.icp0.io/metrics (staging)
pub const BITCOIN_MAINNET_CANISTER_RESPONSE: &str = r#"{
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

// https://mempool.space/api/blocks/tip/height
pub const BITCOIN_MAINNET_MEMPOOL_RESPONSE: &str = r#"700008"#;

// https://ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics
pub const BITCOIN_TESTNET_CANISTER_RESPONSE: &str = r#"{
    # HELP main_chain_height Height of the main chain.
    # TYPE main_chain_height gauge
    main_chain_height 55001 1682533330541
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

// https://mempool.space/testnet4/api/blocks/tip/height
pub const BITCOIN_TESTNET_MEMPOOL_RESPONSE: &str = r#"55002"#;

// https://api.blockchair.com/dogecoin/stats
pub const DOGECOIN_MAINNET_API_BLOCKCHAIR_COM_RESPONSE: &str = r#"{
    "data": {
        "blocks": 5926988,
        "transactions": 406778945,
        "outputs": 965526590,
        "circulation": 15144237638370525000,
        "blocks_24h": 1349,
        "transactions_24h": 29922,
        "difficulty": 51032742.27274,
        "volume_24h": 1214881459793204500,
        "mempool_transactions": 104,
        "mempool_size": 26895,
        "mempool_tps": 0.26666666666666666,
        "mempool_total_fee_usd": 1.5422,
        "best_block_height": 5926987,
        "best_block_hash": "36134366860560c09a6b216cdb6ef58e4ef73792fba514e6e04d074382d0974c",
        "best_block_time": "2025-10-21 12:48:34",
        "blockchain_size": 175195447493,
        "average_transaction_fee_24h": 36043104,
        "inflation_24h": 1349000000000000,
        "median_transaction_fee_24h": 4940000,
        "cdd_24h": 23303407681.90695,
        "mempool_outputs": 387,
        "largest_transaction_24h": {
            "hash": "ced1247b24cdfe7b66496f1a88b01d2f599f48444661c57c8539d65371f0c9e5",
            "value_usd": 120613136
        },
        "nodes": 520,
        "hashrate_24h": "3424749342425088",
        "inflation_usd_24h": 2644606.58,
        "average_transaction_fee_usd_24h": 0.07065962291104853,
        "median_transaction_fee_usd_24h": 0.009684474799999999,
        "market_price_usd": 0.196042,
        "market_price_btc": 0.0000017926790237479,
        "market_price_usd_change_24h_percentage": -2.16725,
        "market_cap_usd": 29679746139,
        "market_dominance_percentage": 0.75,
        "suggested_transaction_fee_per_byte_sat": 500000,
        "hodling_addresses": 8154058
    },
    "context": {
        "code": 200,
        "source": "A",
        "state": 5926987,
        "market_price_usd": 0.195606,
        "cache": {
            "live": false,
            "duration": "Ignore",
            "since": "2025-10-21 12:49:51",
            "until": "2025-10-21 12:51:02",
            "time": 0.0000040531158447265625
        },
        "api": {
            "version": "2.0.95-ie",
            "last_major_update": "2022-11-07 02:00:00",
            "next_major_update": "2023-11-12 02:00:00",
            "documentation": "https://blockchair.com/api/docs",
            "notice": "Try out our new API v.3: https://3xpl.com/data"
        },
        "servers": "API4,DOGE0",
        "time": 0.6437540054321289,
        "render_time": 0.0019309520721435547,
        "full_time": 0.0019350051879882812,
        "request_cost": 1
    }
}"#;

// https://api.blockcypher.com/v1/doge/main
pub const DOGECOIN_MAINNET_API_BLOCKCYPHER_COM_RESPONSE: &str = r#"{
    "name": "DOGE.main",
    "height": 5926989,
    "hash": "bfbcae1f6dcc41710caad2f638dbe9b4006f6c4dd456b99a12253b4152e55cf6",
    "time": "2025-10-21T12:54:01.783276312Z",
    "latest_url": "https://api.blockcypher.com/v1/doge/main/blocks/bfbcae1f6dcc41710caad2f638dbe9b4006f6c4dd456b99a12253b4152e55cf6",
    "previous_hash": "0037287a6dfa3426da3e644da91d00b2d240a829b9b2a30d256b7eef89b78068",
    "previous_url": "https://api.blockcypher.com/v1/doge/main/blocks/0037287a6dfa3426da3e644da91d00b2d240a829b9b2a30d256b7eef89b78068",
    "peer_count": 131,
    "unconfirmed_count": 140012,
    "high_fee_per_kb": 219719686,
    "medium_fee_per_kb": 55929134,
    "low_fee_per_kb": 6664830,
    "last_fork_height": 5925951,
    "last_fork_hash": "5f1e661913de85c9fee78fdd998eefeef3284d28ed3c069e96af6414fa8be377"
}"#;

// https://gordg-fyaaa-aaaan-aaadq-cai.raw.ic0.app/metrics
// https://bhuiy-ciaaa-aaaad-abwea-cai.raw.icp0.io/metrics
pub const DOGECOIN_MAINNET_CANISTER_RESPONSE: &str = r#"
    # HELP main_chain_height Height of the main chain.
    # TYPE main_chain_height gauge
    main_chain_height 5931098 1761310299589
    # HELP stable_height The height of the latest stable block.
    # TYPE stable_height gauge
    stable_height 5930458 1761310299589
    # HELP utxos_length The number of UTXOs in the set.
    # TYPE utxos_length gauge
    utxos_length 202812896 1761310299589
    # HELP address_utxos_length The number of UTXOs that are owned by supported addresses.
    # TYPE address_utxos_length gauge
    address_utxos_length 202383805 1761310299589
"#;

// https://doge-electrs-demo.qed.me/blocks/tip/height
pub const DOGECOIN_MAINNET_PSY_PROTOCOL_RESPONSE: &str = r#"5931072"#;
