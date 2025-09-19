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
