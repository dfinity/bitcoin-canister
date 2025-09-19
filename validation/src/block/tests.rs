use crate::fixtures::{SimpleHeaderStore, MOCK_CURRENT_TIME};
use crate::{BlockValidator, ValidateBlockError};
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
fn should_validate_first_block_after_genesis() {
    // https://learnmeabitcoin.com/explorer/block/00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048
    const BLOCK_1_HEX: &str = "010000006fe28c0ab6f1b372c1a6a246ae63f74f931e8365e15a089c68d6190000000000982051fd1e4ba744bbbe680e1fee14677ba1a3c3540bf7b1cdb606e857233e0e61bc6649ffff001d01e362990101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0704ffff001d0104ffffffff0100f2052a0100000043410496b538e853519c726a2c91e61ec11600ae1390813a627c66fb8be7947be63c52da7589379515d4e0a604f8141781e62294721166bf621e73a82cbf2342c858eeac00000000";
    let network = Network::Bitcoin;
    let validator = BlockValidator::new(SimpleHeaderStore::new_with_genesis(network), network);
    let first_block: bitcoin::Block = deserialize(&hex!(BLOCK_1_HEX)).unwrap();
    assert_eq!(
        first_block.block_hash().to_string(),
        "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048"
    );

    assert_eq!(
        validator.validate_block(&first_block, MOCK_CURRENT_TIME),
        Ok(())
    );
}

#[test]
fn should_prevent_merkle_tree_collision() {
    // https://learnmeabitcoin.com/explorer/block/0000000000001e832afe3c8a7eb78fbf7a72531b0a1304a3f205e36dc9086216
    const BLOCK_HEADER_128460_HEX: &str = "010000005d8f3d3f019ca23cb58651a4d76a2d74ab99d2854133dd75390c000000000000caa4e151516c9e7fdf2393920127c1e927dc6a588879dcbbf680011ff387e10dda8be94d2194261ae905c1cc";
    // https://learnmeabitcoin.com/explorer/block/00000000000008a662b4a95a46e4c54cb04852525ac0ef67d1bcac85238416d4
    const BLOCK_128461_HEX: &str = "01000000166208c96de305f2a304130a1b53727abf8fb77e8a3cfe2a831e000000000000d4fd086755b4d46221362a09a4228bed60d729d22362b87803ff44b72c138ec04a8ce94d2194261af9551f720701000000010000000000000000000000000000000000000000000000000000000000000000ffffffff08042194261a026005ffffffff018076242a01000000434104390e51c3d66d5ee10327395872e33bc232e9e1660225c9f88fa594fdcdcd785d86b1152fb380a63cdf57d8cf2345a55878412a6864656b158704e0b734b3fd9dac000000000100000001f591edc180a889b21a45b6bd5b5e0017d4137dae9695703107ac1e6e878c9f02000000008b483045022100e066df28b29bf18bfcd8da11ea576a6f502f59e7b1d37e2e849ee4648008962b022023be840ec01ffa6860b5577bf0b8546541f40c287eb57b8b421a1396c7aea583014104add16286f51f68cee1b436d0c29a41a59fa8bd224eb6bec34b073512303c70fc3d630cb4952416ef02340c56bee2eef294659b4023ea8a3d90a297bdb54321f9ffffffff02508470b5000000001976a91472579bbeaeca0802fde07ce88f946b64da63989388ac40aeeb02000000001976a914d2a7410246b5ece345aa821af89bff0b6fa3bcaa88ac0000000001000000016197cb143d4cef51389076fdee3f62c294b65bc9aff217a6c71b9dd987e22754000000008c493046022100bf174e942e4619f4e470b5d8b1c0c8ded9e2f7a6616c073c5ab05cc9d699ede3022100a642fa9d0bcc89523635f9468e4813a120b233a249678de0ebf7ba398a4205f6014104122979c0ac1c3af2aa84b4c1d6a9b3b6fa491827f1a2ba37c4b58bdecd644438da715497a44b16aedbadbd18cf9765cdb36851284f643ed743c4365798dd314affffffff02c0404384000000001976a91443cd8fbad7421a53f9e899a2c9761259705d465b88acc0f4f50e000000001976a9142f6c963506b0a2c93a09a92171957e9e7e11a7a388ac00000000010000000228a11f953c26d558a8299ad9dc61279d7abc9a4059820b614bf403c05e471c481d0000008b48304502205baff189016e6fee8e0faa9eebdc8f150d2d3815007719ceccabd995607bb0b0022100f4cc49ef0b29561e976bf6f6f7ae135f665b8dd38a67634bb6bbe74c0da9c1f7014104dd5920aedc3f79ace9c8061f3724812f5b218ea81d175dd990071175874d6c79025f9db516ab23975e510645aabc4ee699cc5c24358a403d15a7736a504399f8ffffffff191b06773a7cec0bb30539f185edbf1d139f9756071c6ae395c1c29f3e2484f6010000008c493046022100c7123436476f923cd8dacbe132f5128b529baa194c9aedc570402d8d2d7902ac02210094e6974695265d96d5859ab493df00c90b62a84dcc33a05753aea23b38c249670141041d878bc5438ff439490e71d059e6b687e511336c0aa53e0d129663c91db71cfe20008891f1e4780bf1139ec9c9e81bfd2e3ea9009608a78d96a5a3a5bf7812baffffffff0200093d00000000001976a914fd0d4c3d0963db8358bd01ba6f386d4c5ef2e30288ac0084d717000000001976a914dcb1e8e699eb9f07a1ddfd5d764aa74359ddd93088ac00000000010000000118e2286c42643e6146669b0f5ee35454fe256aac2b1401dbeefd941f2e6d2074000000008b483045022100edec1c5078fed29d808282d62f167eb3f0ea6a6655f3869c12eca9c63d8463c2022031a3ae430be137932059b4a3e3fb7f1e1f2a05065dbc47c3142972de45c76daa01410423162e5ac10ec46c4a142fea3197cc66e614b9f28f014882ebc8271c4ab6022e474ccdc246445dd2479f9de217e8aaf4d770da15aff1078d329c02e0f4de8d77ffffffff02b00ac165000000001976a914f543a7f0dfcd621a05c646810ba94da791ed14c488ac80de8002000000001976a9144763f6309b3aca0bff49ed6365ffbd791b1afc5d88ac0000000001000000014e3632994e6cbcae4122bf9e8de242aa1d7c13bf6d045392fa69fa92353f13cf000000008c493046022100c6879938322e9945dae2404a2b104b534df7fdab5927a30a57a12418d619c3b8022100c53331f402010cbdc8297d7a827154e42263fc2f6cef6e56b85bbc061d5e30810141047e717e70b8c5e928bc2c482662dbe9007113f7a5fb0360da1d2f193add960fed97ab3163e85c02b127829d694ab4a796326918d4f639d0b19345f7558406667dffffffff0270c8b165000000001976a9146c908731300d5c0a4215ba3bb3041b4f313d14f688ac40420f00000000001976a91457b01e2a6bf178a10a0e36cd3e301a41ac58b68b88ac000000000100000001a2e94f26db15d7098104a3616b650cc7490eca961a23111c12c3d94f593ab3bc000000008c493046022100b355076f2c956d7565d44fdf589ebdbdff70abcd806c71845b47d31c3579cbc00221008352a03c5276ba481ae92a2327307ad1ce9b234be7386c105fb914ceb9c63341014104872ee8390f11c8ac309df772362614ff7c99f98e1fd68888c5e8765d630c93ae86fcd33922b17f5da490ea14a9f9002ef4e7fb11166ba399f9794296ca02e401ffffffff02f07d5460000000001976a914ff1da11fbd50b9906e78c694169c19902d2ee20388ac804a5d05000000001976a91444d5774b8277c59a07ed9dce1225e2d24a3faab188ac00000000";
    let valid_block: bitcoin::Block = deserialize(&hex!(BLOCK_128461_HEX)).unwrap();

    // The Rust implementation is currently subject to
    // [CVE-2012-2459](https://bitcointalk.org/index.php?topic=102395)
    let mut forged_block = valid_block.clone();
    forged_block.txdata.push(forged_block.txdata[6].clone());
    assert!(forged_block.check_merkle_root());

    let store =
        SimpleHeaderStore::new(deserialize(&hex!(BLOCK_HEADER_128460_HEX)).unwrap(), 128460);
    let validator = BlockValidator::new(store, Network::Bitcoin);

    assert_eq!(
        validator.validate_block(&valid_block, MOCK_CURRENT_TIME),
        Ok(())
    );
    assert_eq!(
        validator.validate_block(&forged_block, MOCK_CURRENT_TIME),
        Err(ValidateBlockError::DuplicateTransactions)
    );
}
