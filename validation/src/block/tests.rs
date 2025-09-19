use crate::fixtures::{SimpleHeaderStore, MOCK_CURRENT_TIME};
use crate::BlockValidator;
use bitcoin::consensus::deserialize;
use bitcoin::hashes::Hash;
use bitcoin::{block, CompactTarget, Network, TxMerkleNode};
use hex_lit::hex;
use std::str::FromStr;

// Tests taken from
// https://github.com/rust-bitcoin/rust-bitcoin/blob/674ac57bce47e343d8f7c82e451aed5568766ba0/bitcoin/src/blockdata/block.rs#L537
mod bitcoin_tests {
    use crate::block::validate_block;
    use crate::{BlockValidator, ValidateBlockError};
    use bitcoin::block::Header;
    use bitcoin::consensus::deserialize;
    use bitcoin::hashes::Hash;
    use bitcoin::{
        absolute, transaction, Amount, Block, Network, OutPoint, ScriptBuf, Sequence, Transaction,
        TxIn, TxOut, Txid, Witness,
    };
    use hex_lit::hex;

    #[test]
    fn block_validation_no_transactions() {
        let header = header();
        let transactions = Vec::new(); // Empty transactions

        let block = Block {
            header,
            txdata: transactions,
        };
        match validate_block(&block) {
            Err(ValidateBlockError::NoTransactions) => (),
            other => panic!("Expected NoTransactions error, got: {:?}", other),
        }
    }

    #[test]
    fn block_validation_invalid_coinbase() {
        let header = header();

        // Create a non-coinbase transaction (has a real previous output, not all zeros)
        let non_coinbase_tx = Transaction {
            version: transaction::Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([1; 32]), // Not all zeros
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::ONE_BTC,
                script_pubkey: ScriptBuf::new(),
            }],
        };

        let transactions = vec![non_coinbase_tx];
        let block = Block {
            header,
            txdata: transactions,
        };

        match validate_block(&block) {
            Err(ValidateBlockError::InvalidCoinbase) => (),
            other => panic!("Expected InvalidCoinbase error, got: {:?}", other),
        }
    }

    #[test]
    fn block_validation_success_with_coinbase() {
        // Use the genesis block which has a valid coinbase
        let genesis = bitcoin::constants::genesis_block(Network::Bitcoin);

        assert_eq!(
            validate_block(&genesis),
            Ok(()),
            "Genesis block should validate successfully"
        );
    }

    fn header() -> Header {
        let header = hex!("010000004ddccd549d28f385ab457e98d1b11ce80bfea2c5ab93015ade4973e400000000bf4473e53794beae34e64fccc471dace6ae544180816f89591894e0f417a914cd74d6e49ffff001d323b3a7b");
        deserialize(&header).expect("can't deserialize correct block header")
    }
}

#[test]
fn should_validate_block_after_genesis() {
    let network = Network::Bitcoin;
    let validator = BlockValidator::new(SimpleHeaderStore::new_with_genesis(network), network);
    // https://mempool.space/block/1?showDetails=true&view=actual#details
    let first_block = bitcoin::Block {
        header: deserialize(&hex!("010000006fe28c0ab6f1b372c1a6a246ae63f74f931e8365e15a089c68d6190000000000982051fd1e4ba744bbbe680e1fee14677ba1a3c3540bf7b1cdb606e857233e0e61bc6649ffff001d01e36299")).unwrap(),
        txdata: vec![deserialize(&hex!("01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0704ffff001d0104ffffffff0100f2052a0100000043410496b538e853519c726a2c91e61ec11600ae1390813a627c66fb8be7947be63c52da7589379515d4e0a604f8141781e62294721166bf621e73a82cbf2342c858eeac00000000")).unwrap()],
    };
    assert_eq!(
        first_block.block_hash().to_string(),
        "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048"
    );

    assert_eq!(
        validator.validate_block(&first_block, MOCK_CURRENT_TIME),
        Ok(())
    );
}
