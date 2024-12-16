use bitcoin::blockdata::constants::genesis_block;
use bitcoin::{
    absolute::LockTime,
    block::{Header, Version},
    key::Keypair,
    secp256k1::Secp256k1,
    Address, Amount, Block as BitcoinBlock, BlockHash, CompressedPublicKey, Network, OutPoint,
    PublicKey, Script, Sequence, Target, Transaction, TxIn, TxMerkleNode, TxOut, Witness,
    XOnlyPublicKey,
};
use ic_btc_types::Block;
use simple_rng::{fill_bytes, generate_keypair};
use std::str::FromStr;

mod simple_rng;

/// Generates a random P2PKH address.
pub fn random_p2pkh_address(network: Network) -> Address {
    let secp = Secp256k1::new();
    let (_, pk) = generate_keypair(&secp);

    Address::p2pkh(PublicKey::new(pk), network)
}

/// Generates a random P2SH address.
pub fn random_p2tr_address(network: Network) -> Address {
    let secp = Secp256k1::new();
    let (sk, _) = generate_keypair(&secp);
    let keypair = Keypair::from_secret_key(&secp, &sk);
    let (xonly, _) = XOnlyPublicKey::from_keypair(&keypair);

    Address::p2tr(&secp, xonly, None, network)
}

/// Generates a random P2WPKH address.
pub fn random_p2wpkh_address(network: Network) -> Address {
    let secp = Secp256k1::new();
    let (_, pk) = generate_keypair(&secp);

    Address::p2wpkh(
        &CompressedPublicKey::try_from(PublicKey::new(pk))
            .expect("failed to create p2wpkh address"),
        network,
    )
}

/// Generates a random P2WSH address.
pub fn random_p2wsh_address(network: Network) -> Address {
    let mut bytes = [0u8; 32];
    fill_bytes(&mut bytes);
    Address::p2wsh(&Script::from_bytes(&bytes).to_p2wsh(), network)
}

fn coinbase_input() -> TxIn {
    TxIn {
        previous_output: OutPoint::null(),
        script_sig: Script::new().into(),
        sequence: Sequence(0xffffffff),
        witness: Witness::new(),
    }
}

pub struct BlockBuilder {
    prev_header: Option<Header>,
    transactions: Vec<Transaction>,
}

impl BlockBuilder {
    pub fn genesis() -> Self {
        Self {
            prev_header: None,
            transactions: vec![],
        }
    }

    pub fn with_prev_header(prev_header: Header) -> Self {
        Self {
            prev_header: Some(prev_header),
            transactions: vec![],
        }
    }

    pub fn with_transaction(mut self, transaction: Transaction) -> Self {
        self.transactions.push(transaction);
        self
    }

    pub fn build(self) -> BitcoinBlock {
        let txdata = if self.transactions.is_empty() {
            // Create a random coinbase transaction.
            vec![TransactionBuilder::coinbase().build()]
        } else {
            self.transactions
        };

        let merkle_root = bitcoin::merkle_tree::calculate_root(
            txdata
                .iter()
                .map(|tx| *tx.compute_txid().as_raw_hash())
                .clone(),
        )
        .unwrap();
        let merkle_root = TxMerkleNode::from_raw_hash(merkle_root);

        let header = match self.prev_header {
            None => genesis(merkle_root),
            Some(prev_header) => header(&prev_header, merkle_root),
        };

        BitcoinBlock { header, txdata }
    }
}

/// Builds a random chain with the given number of block and transactions
/// starting with the Regtest genesis block.
pub fn build_regtest_chain(num_blocks: u32, num_transactions_per_block: u32) -> Vec<Block> {
    let bitcoin_network = Network::Regtest;
    let genesis_block = Block::new(genesis_block(bitcoin_network));

    // Use a static address to send outputs to.
    // `random_p2pkh_address` isn't used here as it doesn't work in wasm.
    let address = Address::from_str("bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8")
        .unwrap()
        .assume_checked();
    let mut blocks = vec![genesis_block.clone()];
    let mut prev_block: Block = genesis_block;
    let mut value = 1;

    // Since we start with a genesis block, we need `num_blocks - 1` additional blocks.
    for _ in 0..num_blocks - 1 {
        let mut block_builder = BlockBuilder::with_prev_header(*prev_block.header());
        let mut transactions = vec![];
        for _ in 0..num_transactions_per_block {
            transactions.push(
                TransactionBuilder::coinbase()
                    .with_output(&address, value)
                    .build(),
            );
            // Vary the value of the transaction to ensure that
            // we get unique outpoints in the blockchain.
            value += 1;
        }

        for transaction in transactions.iter() {
            block_builder = block_builder.with_transaction(transaction.clone());
        }

        let block = Block::new(block_builder.build());
        blocks.push(block.clone());
        prev_block = block;
    }

    blocks
}

fn genesis(merkle_root: TxMerkleNode) -> Header {
    let target = Target::MAX_ATTAINABLE_REGTEST;
    let bits = target.to_compact_lossy();

    let mut header = Header {
        version: Version::from_consensus(1),
        time: 0,
        nonce: 0,
        bits,
        merkle_root,
        prev_blockhash: BlockHash::from_raw_hash(bitcoin::hashes::Hash::all_zeros()),
    };
    solve(&mut header);

    header
}

pub struct TransactionBuilder {
    input: Vec<TxIn>,
    output: Vec<TxOut>,
    lock_time: u32,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            input: vec![],
            output: vec![],
            lock_time: 0,
        }
    }

    pub fn coinbase() -> Self {
        Self {
            input: vec![coinbase_input()],
            output: vec![],
            lock_time: 0,
        }
    }

    pub fn with_input(mut self, previous_output: OutPoint, witness: Option<Witness>) -> Self {
        if self.input == vec![coinbase_input()] {
            panic!("A call `with_input` should not be possible if `coinbase` was called");
        }

        let witness = witness.map_or(Witness::new(), |w| w);
        let input = TxIn {
            previous_output,
            script_sig: Script::new().into(),
            sequence: Sequence(0xffffffff),
            witness,
        };
        self.input.push(input);
        self
    }

    pub fn with_output(mut self, address: &Address, satoshi: u64) -> Self {
        self.output.push(TxOut {
            value: Amount::from_sat(satoshi),
            script_pubkey: address.script_pubkey(),
        });
        self
    }

    pub fn with_lock_time(mut self, time: u32) -> Self {
        self.lock_time = time;
        self
    }

    pub fn build(self) -> Transaction {
        let input = if self.input.is_empty() {
            // Default to coinbase if no inputs provided.
            vec![coinbase_input()]
        } else {
            self.input
        };
        let output = if self.output.is_empty() {
            // Use default of 50 BTC.
            vec![TxOut {
                value: Amount::from_sat(50_0000_0000),
                script_pubkey: random_p2pkh_address(Network::Regtest).script_pubkey(),
            }]
        } else {
            self.output
        };

        Transaction {
            version: bitcoin::transaction::Version(1),
            lock_time: LockTime::from_consensus(self.lock_time),
            input,
            output,
        }
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn header(prev_header: &Header, merkle_root: TxMerkleNode) -> Header {
    let time = prev_header.time + 60 * 10; // 10 minutes.
    let bits = prev_header.target().to_compact_lossy();

    let mut header = Header {
        version: Version::from_consensus(1),
        time,
        nonce: 0,
        bits,
        merkle_root,
        prev_blockhash: prev_header.block_hash(),
    };
    solve(&mut header);

    header
}

fn solve(header: &mut Header) {
    let target = header.target();
    while header.validate_pow(target).is_err() {
        header.nonce += 1;
    }
}

#[cfg(test)]
mod test {
    mod transaction_builder {
        use crate::{random_p2pkh_address, TransactionBuilder};
        use bitcoin::{Network, OutPoint};

        #[test]
        fn new_build() {
            let tx = TransactionBuilder::new().build();
            assert!(tx.is_coinbase());
            assert_eq!(tx.input.len(), 1);
            assert_eq!(tx.input[0].previous_output, OutPoint::null());
            assert_eq!(tx.output.len(), 1);
            assert_eq!(tx.output[0].value.to_sat(), 50_0000_0000);
        }

        #[test]
        fn coinbase() {
            let tx = TransactionBuilder::coinbase().build();
            assert!(tx.is_coinbase());
            assert_eq!(tx.input.len(), 1);
            assert_eq!(tx.input[0].previous_output, OutPoint::null());
            assert_eq!(tx.output.len(), 1);
            assert_eq!(tx.output[0].value.to_sat(), 50_0000_0000);
        }

        #[test]
        #[should_panic(
            expected = "A call `with_input` should not be possible if `coinbase` was called"
        )]
        fn with_input_panic() {
            let address = random_p2pkh_address(Network::Regtest);
            let coinbase_tx = TransactionBuilder::coinbase()
                .with_output(&address, 1000)
                .build();

            TransactionBuilder::coinbase()
                .with_input(bitcoin::OutPoint::new(coinbase_tx.compute_txid(), 0), None);
        }

        #[test]
        fn with_output() {
            let address = random_p2pkh_address(Network::Regtest);
            let tx = TransactionBuilder::coinbase()
                .with_output(&address, 1000)
                .build();

            assert!(tx.is_coinbase());
            assert_eq!(tx.input.len(), 1);
            assert_eq!(tx.input[0].previous_output, OutPoint::null());
            assert_eq!(tx.output.len(), 1);
            assert_eq!(tx.output[0].value.to_sat(), 1000);
            assert_eq!(tx.output[0].script_pubkey, address.script_pubkey());
        }

        #[test]
        fn with_output_2() {
            let network = Network::Regtest;
            let address_0 = random_p2pkh_address(network);
            let address_1 = random_p2pkh_address(network);
            let tx = TransactionBuilder::coinbase()
                .with_output(&address_0, 1000)
                .with_output(&address_1, 2000)
                .build();

            assert!(tx.is_coinbase());
            assert_eq!(tx.input.len(), 1);
            assert_eq!(tx.input[0].previous_output, OutPoint::null());
            assert_eq!(tx.output.len(), 2);
            assert_eq!(tx.output[0].value.to_sat(), 1000);
            assert_eq!(tx.output[0].script_pubkey, address_0.script_pubkey());
            assert_eq!(tx.output[1].value.to_sat(), 2000);
            assert_eq!(tx.output[1].script_pubkey, address_1.script_pubkey());
        }

        #[test]
        fn with_input() {
            let network = Network::Regtest;
            let address = random_p2pkh_address(network);
            let coinbase_tx = TransactionBuilder::coinbase()
                .with_output(&address, 1000)
                .build();

            let tx = TransactionBuilder::new()
                .with_input(bitcoin::OutPoint::new(coinbase_tx.compute_txid(), 0), None)
                .build();
            assert!(!tx.is_coinbase());
            assert_eq!(tx.input.len(), 1);
            assert_eq!(
                tx.input[0].previous_output,
                bitcoin::OutPoint::new(coinbase_tx.compute_txid(), 0)
            );
            assert_eq!(tx.output.len(), 1);
            assert_eq!(tx.output[0].value.to_sat(), 50_0000_0000);
        }

        #[test]
        fn with_input_2() {
            let network = Network::Regtest;
            let address = random_p2pkh_address(network);
            let coinbase_tx_0 = TransactionBuilder::coinbase()
                .with_output(&address, 1000)
                .build();
            let coinbase_tx_1 = TransactionBuilder::coinbase()
                .with_output(&address, 2000)
                .build();

            let tx = TransactionBuilder::new()
                .with_input(
                    bitcoin::OutPoint::new(coinbase_tx_0.compute_txid(), 0),
                    None,
                )
                .with_input(
                    bitcoin::OutPoint::new(coinbase_tx_1.compute_txid(), 0),
                    None,
                )
                .build();
            assert!(!tx.is_coinbase());
            assert_eq!(tx.input.len(), 2);
            assert_eq!(
                tx.input[0].previous_output,
                bitcoin::OutPoint::new(coinbase_tx_0.compute_txid(), 0)
            );
            assert_eq!(
                tx.input[1].previous_output,
                bitcoin::OutPoint::new(coinbase_tx_1.compute_txid(), 0)
            );
            assert_eq!(tx.output.len(), 1);
            assert_eq!(tx.output[0].value.to_sat(), 50_0000_0000);
        }
    }
}
