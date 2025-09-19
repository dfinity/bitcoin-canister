use bitcoin::{
    block::Version, blockdata::constants::genesis_block, consensus::deserialize,
    hashes::hex::FromHex, TxMerkleNode,
};
use csv::Reader;
use proptest::prelude::*;
use std::{path::PathBuf, str::FromStr};

use super::*;
use crate::constants::test::{
    MAINNET_HEADER_586656, MAINNET_HEADER_705600, MAINNET_HEADER_705601, MAINNET_HEADER_705602,
    TESTNET_HEADER_2132555, TESTNET_HEADER_2132556,
};
use crate::fixtures::SimpleHeaderStore;

const MOCK_CURRENT_TIME: Duration = Duration::from_secs(2_634_590_600);

#[test]
fn test_simple_mainnet() {
    let header_705600 = deserialize_header(MAINNET_HEADER_705600);
    let header_705601 = deserialize_header(MAINNET_HEADER_705601);
    let store = SimpleHeaderStore::new(header_705600, 705_600);
    let validator = HeaderValidator::new(&store, Network::Bitcoin);
    let result = validator.validate_header(&header_705601, MOCK_CURRENT_TIME);
    assert!(result.is_ok());
}

#[test]
fn test_simple_testnet() {
    let header_2132555 = deserialize_header(TESTNET_HEADER_2132555);
    let header_2132556 = deserialize_header(TESTNET_HEADER_2132556);
    let store = SimpleHeaderStore::new(header_2132555, 2_132_555);
    let validator = HeaderValidator::new(&store, Network::Testnet);
    let result = validator.validate_header(&header_2132556, MOCK_CURRENT_TIME);
    assert!(result.is_ok());
}

#[test]
fn test_is_header_valid() {
    let header_586656 = deserialize_header(MAINNET_HEADER_586656);
    let mut store = SimpleHeaderStore::new(header_586656, 586_656);
    let headers = get_bitcoin_headers();
    for (i, header) in headers.iter().enumerate() {
        let validator = HeaderValidator::new(&store, Network::Bitcoin);
        let result = validator.validate_header(header, MOCK_CURRENT_TIME);
        assert!(
            result.is_ok(),
            "Failed to validate header on line {}: {:?}",
            i,
            result
        );
        store.add(*header);
    }
}

#[test]
fn test_timestamp_is_less_than_2h_in_future() {
    // Time is represented as the number of seconds after 01.01.1970 00:00.
    // Hence, if block time is 10 seconds after that time,
    // 'timestamp_is_less_than_2h_in_future' should return true.

    assert!(
        timestamp_is_less_than_2h_in_future(Duration::from_secs(10), MOCK_CURRENT_TIME).is_ok()
    );

    assert!(
        timestamp_is_less_than_2h_in_future(MOCK_CURRENT_TIME - ONE_HOUR, MOCK_CURRENT_TIME)
            .is_ok()
    );

    assert!(timestamp_is_less_than_2h_in_future(MOCK_CURRENT_TIME, MOCK_CURRENT_TIME).is_ok());

    assert!(
        timestamp_is_less_than_2h_in_future(MOCK_CURRENT_TIME + ONE_HOUR, MOCK_CURRENT_TIME)
            .is_ok()
    );

    assert!(timestamp_is_less_than_2h_in_future(
        MOCK_CURRENT_TIME + 2 * ONE_HOUR - Duration::from_secs(5),
        MOCK_CURRENT_TIME
    )
    .is_ok());

    // 'timestamp_is_less_than_2h_in_future' should return false
    // because the time is more than 2 hours from the current time.
    assert_eq!(
        timestamp_is_less_than_2h_in_future(
            MOCK_CURRENT_TIME + 2 * ONE_HOUR + Duration::from_secs(10),
            MOCK_CURRENT_TIME
        ),
        Err(ValidateHeaderError::HeaderIsTooFarInFuture {
            block_time: (MOCK_CURRENT_TIME + 2 * ONE_HOUR).as_secs() + 10,
            max_allowed_time: (MOCK_CURRENT_TIME + 2 * ONE_HOUR).as_secs()
        })
    );
}

#[test]
fn test_is_timestamp_valid() {
    let header_705600 = deserialize_header(MAINNET_HEADER_705600);
    let header_705601 = deserialize_header(MAINNET_HEADER_705601);
    let header_705602 = deserialize_header(MAINNET_HEADER_705602);
    let mut store = SimpleHeaderStore::new(header_705600, 705_600);
    store.add(header_705601);
    store.add(header_705602);
    let validator = HeaderValidator::new(&store, Network::Bitcoin);

    let mut header = Header {
        version: Version::from_consensus(0x20800004),
        prev_blockhash: BlockHash::from_str(
            "00000000000000000001eea12c0de75000c2546da22f7bf42d805c1d2769b6ef",
        )
        .unwrap(),
        merkle_root: TxMerkleNode::from_str(
            "c120ff2ae1363593a0b92e0d281ec341a0cc989b4ee836dc3405c9f4215242a6",
        )
        .unwrap(),
        time: 1634590600,
        bits: CompactTarget::from_consensus(0x170e0408),
        nonce: 0xb48e8b0a,
    };
    assert!(validator
        .is_timestamp_valid(&header, MOCK_CURRENT_TIME)
        .is_ok());

    // Monday, October 18, 2021 20:26:40
    header.time = 1634588800;
    assert!(matches!(
        validator.is_timestamp_valid(&header, MOCK_CURRENT_TIME),
        Err(ValidateHeaderError::HeaderIsOld)
    ));

    let result = validator.validate_header(&header, MOCK_CURRENT_TIME);
    assert!(matches!(result, Err(ValidateHeaderError::HeaderIsOld)));

    header.time = (MOCK_CURRENT_TIME - ONE_HOUR).as_secs() as u32;

    assert!(validator
        .is_timestamp_valid(&header, MOCK_CURRENT_TIME)
        .is_ok());

    header.time = (MOCK_CURRENT_TIME + 2 * ONE_HOUR + Duration::from_secs(10)).as_secs() as u32;
    assert_eq!(
        validator.is_timestamp_valid(&header, MOCK_CURRENT_TIME),
        Err(ValidateHeaderError::HeaderIsTooFarInFuture {
            block_time: header.time as u64,
            max_allowed_time: (MOCK_CURRENT_TIME + 2 * ONE_HOUR).as_secs()
        })
    );

    let result = validator.validate_header(&header, MOCK_CURRENT_TIME);
    assert_eq!(
        result,
        Err(ValidateHeaderError::HeaderIsTooFarInFuture {
            block_time: header.time as u64,
            max_allowed_time: (MOCK_CURRENT_TIME + 2 * ONE_HOUR).as_secs(),
        })
    );
}

#[test]
fn test_is_header_valid_missing_prev_header() {
    let header_705600 = deserialize_header(MAINNET_HEADER_705600);
    let header_705602 = deserialize_header(MAINNET_HEADER_705602);
    let store = SimpleHeaderStore::new(header_705600, 705_600);
    let validator = HeaderValidator::new(&store, Network::Bitcoin);
    let result = validator.validate_header(&header_705602, MOCK_CURRENT_TIME);
    assert!(matches!(
        result,
        Err(ValidateHeaderError::PrevHeaderNotFound)
    ));
}

#[test]
fn test_is_header_valid_invalid_header_target() {
    let header_705600 = deserialize_header(MAINNET_HEADER_705600);
    let mut header = deserialize_header(MAINNET_HEADER_705601);
    header.bits = pow_limit_bits(&Network::Bitcoin);
    let store = SimpleHeaderStore::new(header_705600, 705_600);
    let validator = HeaderValidator::new(&store, Network::Bitcoin);
    let result = validator.validate_header(&header, MOCK_CURRENT_TIME);
    assert!(matches!(
        result,
        Err(ValidateHeaderError::InvalidPoWForHeaderTarget)
    ));
}

#[test]
fn test_is_header_valid_invalid_computed_target() {
    let pow_bitcoin = pow_limit_bits(&Network::Bitcoin);
    let pow_regtest = pow_limit_bits(&Network::Regtest);
    let h0 = genesis_header(Network::Bitcoin, pow_bitcoin);
    let h1 = next_block_header(h0, pow_regtest);
    let h2 = next_block_header(h1, pow_regtest);
    let h3 = next_block_header(h2, pow_regtest);
    let mut store = SimpleHeaderStore::new(h0, 0);
    store.add(h1);
    store.add(h2);
    let validator = HeaderValidator::new(&store, Network::Regtest);
    let result = validator.validate_header(&h3, MOCK_CURRENT_TIME);
    assert!(matches!(
        result,
        Err(ValidateHeaderError::InvalidPoWForComputedTarget)
    ));
}

#[test]
fn test_is_header_valid_target_difficulty_above_max() {
    let header_705600 = deserialize_header(MAINNET_HEADER_705600);
    let mut header = deserialize_header(MAINNET_HEADER_705601);
    header.bits = pow_limit_bits(&Network::Regtest);
    let store = SimpleHeaderStore::new(header_705600, 705_600);
    let validator = HeaderValidator::new(&store, Network::Bitcoin);
    let result = validator.validate_header(&header, MOCK_CURRENT_TIME);
    assert!(matches!(
        result,
        Err(ValidateHeaderError::TargetDifficultyAboveMax)
    ));
}

fn test_next_targets(network: Network, headers_path: &str, up_to_height: usize) {
    use bitcoin::consensus::Decodable;
    use std::io::BufRead;
    let file = std::fs::File::open(
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(headers_path),
    )
    .unwrap();

    let rdr = std::io::BufReader::new(file);

    println!("Loading headers...");
    let mut headers = vec![];
    for line in rdr.lines() {
        let header = line.unwrap();
        // If this line fails make sure you install git-lfs.
        let decoded = hex::decode(header.trim()).unwrap();
        let header = Header::consensus_decode(&mut &decoded[..]).unwrap();
        headers.push(header);
    }

    println!("Creating header store...");
    let mut store = SimpleHeaderStore::new(headers[0], 0);
    for header in headers[1..].iter() {
        store.add(*header);
    }

    let validator = HeaderValidator::new(&store, network);
    println!("Verifying next targets...");
    proptest!(|(i in 0..up_to_height)| {
        // Compute what the target of the next header should be.
        let expected_next_target =
            validator.get_next_target(&headers[i], i as u32, headers[i + 1].time);

        // Assert that the expected next target matches the next header's target.
        assert_eq!(
            expected_next_target,
            Target::from_compact(headers[i + 1].bits)
        );
    });
}

#[test]
fn mainnet_next_targets() {
    test_next_targets(
        Network::Bitcoin,
        "tests/data/block_headers_mainnet.csv",
        700_000,
    );
}

#[test]
fn testnet_next_targets() {
    test_next_targets(
        Network::Testnet,
        "tests/data/block_headers_testnet.csv",
        2_400_000,
    );
}

fn genesis_header(network: Network, bits: CompactTarget) -> Header {
    Header {
        bits,
        ..genesis_block(network).header
    }
}

fn next_block_header(prev: Header, bits: CompactTarget) -> Header {
    Header {
        prev_blockhash: prev.block_hash(),
        time: prev.time + TEN_MINUTES,
        bits,
        ..prev
    }
}

/// Creates a chain of headers with the given length and
/// proof of work for the first header.
fn create_chain(
    network: &Network,
    initial_pow: CompactTarget,
    chain_length: u32,
) -> (SimpleHeaderStore, Header) {
    let pow_limit = pow_limit_bits(network);
    let h0 = genesis_header(*network, initial_pow);
    let mut store = SimpleHeaderStore::new(h0, 0);
    let mut last_header = h0;

    for _ in 1..chain_length {
        let new_header = next_block_header(last_header, pow_limit);
        store.add(new_header);
        last_header = new_header;
    }

    (store, last_header)
}

#[test]
fn test_next_target_regtest() {
    // This test checks the chain of headers of different lengths
    // with non-limit PoW in the first block header and PoW limit
    // in all the other headers.
    // Expect difficulty to be equal to the non-limit PoW.

    // Arrange.
    let network = Network::Regtest;
    let expected_pow = CompactTarget::from_consensus(7); // Some non-limit PoW, the actual value is not important.
    for chain_length in 1..10 {
        let (store, last_header) = create_chain(&network, expected_pow, chain_length);
        assert_eq!(store.height() + 1, chain_length);
        // Act.
        let validator = HeaderValidator::new(&store, Network::Regtest);
        let target = validator.get_next_target(
            &last_header,
            chain_length - 1,
            last_header.time + TEN_MINUTES,
        );
        // Assert.
        assert_eq!(target, Target::from_compact(expected_pow));
    }
}

#[test]
fn test_compute_next_difficulty_for_backdated_blocks() {
    // Arrange: Set up the test network and parameters
    let network = Network::Testnet;
    let chain_length = DIFFICULTY_ADJUSTMENT_INTERVAL - 1; // To trigger the difficulty adjustment.
    let genesis_difficulty = CompactTarget::from_consensus(486604799);

    // Create the genesis header and initialize the header store
    let genesis_header = genesis_header(network, genesis_difficulty);
    let mut store = SimpleHeaderStore::new(genesis_header, 0);
    let mut last_header = genesis_header;
    for _ in 1..chain_length {
        let new_header = Header {
            prev_blockhash: last_header.block_hash(),
            time: last_header.time - 1, // Each new block is 1 second earlier
            ..last_header
        };
        store.add(new_header);
        last_header = new_header;
    }

    // Act.
    let validator = HeaderValidator::new(&store, Network::Testnet);
    let difficulty = validator.compute_next_difficulty(&last_header, chain_length);

    // Assert.
    assert_eq!(difficulty, CompactTarget::from_consensus(473956288));
}

fn deserialize_header(encoded_bytes: &str) -> Header {
    let bytes = Vec::from_hex(encoded_bytes).expect("failed to decoded bytes");
    deserialize(bytes.as_slice()).expect("failed to deserialize")
}

/// This function reads `num_headers` headers from `tests/data/headers.csv`
/// and returns them.
fn get_bitcoin_headers() -> Vec<Header> {
    let rdr = Reader::from_path(
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("tests/data/headers.csv"),
    );
    assert!(rdr.is_ok(), "Unable to find blockchain_headers.csv file");
    let mut rdr = rdr.unwrap();
    let mut headers = vec![];
    for result in rdr.records() {
        let record = result.unwrap();
        let header = Header {
            version: Version::from_consensus(
                i32::from_str_radix(record.get(0).unwrap(), 16).unwrap(),
            ),
            prev_blockhash: BlockHash::from_str(record.get(1).unwrap()).unwrap(),
            merkle_root: TxMerkleNode::from_str(record.get(2).unwrap()).unwrap(),
            time: u32::from_str_radix(record.get(3).unwrap(), 16).unwrap(),
            bits: CompactTarget::from_consensus(
                u32::from_str_radix(record.get(4).unwrap(), 16).unwrap(),
            ),
            nonce: u32::from_str_radix(record.get(5).unwrap(), 16).unwrap(),
        };
        headers.push(header);
    }
    headers
}
