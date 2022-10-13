use crate::{
    memory::Memory,
    runtime::{inc_performance_counter, performance_counter, print},
    state::{BlockIngestionStats, OUTPOINT_SIZE},
    types::{Address, Block, Network, OutPoint, Slicing, Storable, Transaction, TxOut, Txid},
};
use bitcoin::{Script, TxOut as BitcoinTxOut};
use ic_btc_types::{Height, Satoshi};
use ic_stable_structures::{btreemap::Iter, StableBTreeMap, Storable as _};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
mod utxos;
use utxos::Utxos;

lazy_static::lazy_static! {
    pub static ref DUPLICATE_TX_IDS: [Txid; 2] = [
        Txid::from_str("d5d27987d2a3dfc724e359870c6644b40e497bdc0589a033220fe15429d88599").unwrap(),
        Txid::from_str("e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468").unwrap(),
    ];
}

// The threshold at which time slicing kicks in.
// At the time of this writing it is equivalent to 80% of the maximum instructions limit.
const MAX_INSTRUCTIONS_THRESHOLD: u64 = 4_000_000_000;

// The longest addresses are bech32 addresses, and a bech32 string can be at most 90 chars.
// See https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki
const MAX_ADDRESS_SIZE: u32 = 90;

const MAX_ADDRESS_OUTPOINT_SIZE: u32 = MAX_ADDRESS_SIZE + OUTPOINT_SIZE;

#[derive(Serialize, Deserialize)]
pub struct UtxoSet {
    utxos: Utxos,

    network: Network,

    // An index for fast retrievals of an address's UTXOs.
    // NOTE: Stable structures don't need to be serialized.
    #[serde(skip, default = "init_address_outpoints")]
    address_to_outpoints: StableBTreeMap<Memory, Vec<u8>, Vec<u8>>,

    // A map of an address and its current balance.
    // NOTE: Stable structures don't need to be serialized.
    #[serde(skip, default = "init_balances")]
    balances: StableBTreeMap<Memory, Address, u64>,

    // The height of the block that will be ingested next.
    // NOTE: The `next_height` is stored, rather than the current height, because:
    //   * The `UtxoSet` is initialized as empty with no blocks.
    //   * The height of the genesis block is defined as zero.
    //
    // Rather than making this an optional to handle the case where the UTXO set is empty, we
    // instead store the `next_height` to avoid having this special case.
    next_height: Height,

    /// A stable block that has partially been written to the UTXO set. Used for time slicing.
    pub partial_stable_block: Option<PartialStableBlock>,
}

impl UtxoSet {
    pub fn new(network: Network) -> Self {
        Self {
            utxos: Utxos::default(),
            balances: init_balances(),
            address_to_outpoints: init_address_outpoints(),
            network,
            next_height: 0,
            partial_stable_block: None,
        }
    }

    /// Ingests a block into the `UtxoSet`.
    ///
    /// The inputs of all the transactions in the block are removed and the outputs are inserted.
    /// The block is assumed to be valid, and so a failure of any of these operations causes a panic.
    ///
    /// Returns `Slicing::Done` if ingestion is complete, or `Slicing::Paused` if ingestion hasn't
    /// fully completed due to instruction limits. In the latter case, one or more calls to
    /// `ingest_block_continue` are necessary to finish the block ingestion.
    pub fn ingest_block(&mut self, block: Block) -> Slicing<()> {
        assert!(
            self.partial_stable_block.is_none(),
            "Cannot ingest new block while previous block (height {}) isn't fully ingested",
            self.next_height
        );

        self.ingest_block_helper(block, 0, 0, 0, BlockIngestionStats::default())
    }

    /// Continue ingesting a block.
    pub fn ingest_block_continue(&mut self) -> Slicing<()> {
        match self.partial_stable_block.take() {
            Some(p) => self.ingest_block_helper(
                p.block,
                p.next_tx_idx,
                p.next_input_idx,
                p.next_output_idx,
                p.stats,
            ),
            None => {
                // No partially ingested block found. Nothing to do.
                Slicing::Done
            }
        }
    }

    /// Returns the balance of the given address.
    pub fn get_balance(&self, address: &Address) -> Satoshi {
        self.balances.get(address).unwrap_or(0)
    }

    /// Returns the UTXO of the given outpoint.
    pub fn get_utxo(&self, outpoint: &OutPoint) -> Option<(TxOut, Height)> {
        self.utxos.get(outpoint)
    }

    /// Returns the outpoints owned by the given address.
    /// An optional offset can be specified for pagination.
    pub fn get_address_outpoints(
        &self,
        address: &Address,
        offset: &Option<(Height, OutPoint)>,
    ) -> Iter<Memory, Vec<u8>, Vec<u8>> {
        self.address_to_outpoints.range(
            address.to_bytes().to_vec(),
            offset.as_ref().map(|x| x.to_bytes()),
        )
    }

    /// Returns the number of UTXOs in the set.
    pub fn utxos_len(&self) -> u64 {
        self.utxos.len()
    }

    /// Returns the number of UTXOs that are owned by supported addresses.
    pub fn address_owned_utxos_len(&self) -> u64 {
        self.address_to_outpoints.len()
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub fn next_height(&self) -> Height {
        self.next_height
    }

    // Ingests a block starting from the given transaction and input/output indices.
    fn ingest_block_helper(
        &mut self,
        block: Block,
        next_tx_idx: usize,
        mut next_input_idx: usize,
        mut next_output_idx: usize,
        mut stats: BlockIngestionStats,
    ) -> Slicing<()> {
        let ins_start = performance_counter();
        stats.num_rounds += 1;
        for (tx_idx, tx) in block.txdata().iter().enumerate().skip(next_tx_idx) {
            if let Slicing::Paused((next_input_idx, next_output_idx)) =
                self.ingest_tx_with_slicing(tx, next_input_idx, next_output_idx, &mut stats)
            {
                stats.ins_total += performance_counter() - ins_start;

                // Getting close to the the instructions limit. Pause execution.
                self.partial_stable_block = Some(PartialStableBlock {
                    block,
                    next_tx_idx: tx_idx,
                    next_input_idx,
                    next_output_idx,
                    stats,
                });

                return Slicing::Paused(());
            }

            // Current transaction was processed in full. Reset the indices for next transaction.
            next_input_idx = 0;
            next_output_idx = 0;
        }

        stats.ins_total += performance_counter() - ins_start;
        print(&format!(
            "[INSTRUCTION COUNT] Ingest Block {}: {:?}",
            self.next_height, stats
        ));

        // Block ingestion complete.
        self.next_height += 1;
        Slicing::Done
    }

    // Ingests a transaction into the given UTXO set.
    //
    // NOTE: This method does a form of time-slicing to stay within the instruction limit, and
    // multiple calls may be required for the transaction to be ingested.
    //
    // Returns a `Slicing` struct with a tuple containing (# inputs removed, # outputs inserted).
    fn ingest_tx_with_slicing(
        &mut self,
        tx: &Transaction,
        start_input_idx: usize,
        start_output_idx: usize,
        stats: &mut BlockIngestionStats,
    ) -> Slicing<(usize, usize)> {
        let ins_start = performance_counter();
        let res = self.remove_inputs(tx, start_input_idx);
        stats.ins_remove_inputs += performance_counter() - ins_start;
        if let Slicing::Paused(input_idx) = res {
            return Slicing::Paused((input_idx, 0));
        }

        let ins_start = performance_counter();
        let res = self.insert_outputs(tx, start_output_idx, stats);
        stats.ins_insert_outputs += performance_counter() - ins_start;
        if let Slicing::Paused(output_idx) = res {
            return Slicing::Paused((tx.input().len(), output_idx));
        }

        Slicing::Done
    }

    // Iterates over transaction inputs, starting from `start_idx`, and removes them from the UTXO set.
    fn remove_inputs(&mut self, tx: &Transaction, start_idx: usize) -> Slicing<usize> {
        if tx.is_coin_base() {
            return Slicing::Done;
        }

        for (input_idx, input) in tx.input().iter().enumerate().skip(start_idx) {
            // NOTE: We're using `inc_performance_counter` here to increment the mock performance
            // counter in the unit tests.
            if inc_performance_counter() >= MAX_INSTRUCTIONS_THRESHOLD {
                return Slicing::Paused(input_idx);
            }

            // Remove the input from the UTXOs. The input *must* exist in the UTXO set.
            match self.utxos.remove(&(&input.previous_output).into()) {
                Some((txout, height)) => {
                    if let Ok(address) =
                        Address::from_script(&Script::from(txout.script_pubkey), self.network)
                    {
                        let found = self.address_to_outpoints.remove(
                            &(address.clone(), height, (&input.previous_output).into()).to_bytes(),
                        );

                        assert!(
                            found.is_some(),
                            "Outpoint {:?} not found in the index.",
                            input.previous_output
                        );

                        // Update the balance of the address.
                        let address_balance = self.balances.get(&address).unwrap_or_else(|| {
                            panic!("Address {} must exist in the balances map", address);
                        });

                        match address_balance - txout.value {
                            // Remove the address from the map if balance is zero.
                            0 => self.balances.remove(&address),
                            // Update the balance in the map.
                            balance => self.balances.insert(address, balance).unwrap(),
                        };
                    }
                }
                None => {
                    panic!("Outpoint {:?} not found.", input.previous_output);
                }
            }
        }

        Slicing::Done
    }

    // Iterates over transaction outputs, starting from `start_idx`, and inserts them into the UTXO set.
    fn insert_outputs(
        &mut self,
        tx: &Transaction,
        start_idx: usize,
        stats: &mut BlockIngestionStats,
    ) -> Slicing<usize> {
        for (vout, output) in tx.output().iter().enumerate().skip(start_idx) {
            // NOTE: We're using `inc_performance_counter` here to increment the mock performance
            // counter in the unit tests.
            if inc_performance_counter() >= MAX_INSTRUCTIONS_THRESHOLD {
                return Slicing::Paused(vout);
            }

            if !(output.script_pubkey.is_provably_unspendable()) {
                let ins_start = performance_counter();
                let txid = tx.txid();
                stats.ins_txids += performance_counter() - ins_start;

                let ins_start = performance_counter();
                self.insert_utxo(OutPoint::new(txid, vout as u32), output.clone());
                stats.ins_insert_utxos += performance_counter() - ins_start;
            }
        }

        Slicing::Done
    }

    // Inserts a UTXO into the given UTXO set.
    // A UTXO is represented by the the tuple: (outpoint, output)
    fn insert_utxo(&mut self, outpoint: OutPoint, output: BitcoinTxOut) {
        // Insert the outpoint.
        if let Ok(address) = Address::from_script(&output.script_pubkey, self.network) {
            // Add the address to the index if we can parse it.
            self.address_to_outpoints
                .insert(
                    (address.clone(), self.next_height, outpoint.clone()).to_bytes(),
                    vec![],
                )
                .expect("insertion must succeed");

            // Update the balance of the address.
            let address_balance = self.balances.get(&address).unwrap_or(0);
            self.balances
                .insert(address, address_balance + output.value)
                .expect("insertion must succeed");
        }

        let outpoint_already_exists = self
            .utxos
            .insert(outpoint.clone(), ((&output).into(), self.next_height));

        // Verify that we aren't overwriting a previously seen outpoint.
        // NOTE: There was a bug where there were duplicate transactions. These transactions
        // we overwrite.
        //
        // See: https://en.bitcoin.it/wiki/BIP_0030
        //      https://bitcoinexplorer.org/tx/d5d27987d2a3dfc724e359870c6644b40e497bdc0589a033220fe15429d88599
        //      https://bitcoinexplorer.org/tx/e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468
        if outpoint_already_exists && !DUPLICATE_TX_IDS.contains(&outpoint.txid) {
            panic!(
                "Cannot insert outpoint {:?} because it was already inserted. Block height: {}",
                outpoint, self.next_height
            );
        }
    }

    #[cfg(test)]
    pub fn get_total_supply(&self) -> Satoshi {
        self.utxos.iter().map(|(_, (v, _))| v.value).sum()
    }
}

fn init_address_outpoints() -> StableBTreeMap<Memory, Vec<u8>, Vec<u8>> {
    StableBTreeMap::init(
        crate::memory::get_address_outpoints_memory(),
        MAX_ADDRESS_OUTPOINT_SIZE,
        0, // No values are stored in the map.
    )
}

fn init_balances() -> StableBTreeMap<Memory, Address, u64> {
    // A balance is a u64, which requires 8 bytes.
    const BALANCE_SIZE: u32 = 8;

    StableBTreeMap::init(
        crate::memory::get_balances_memory(),
        MAX_ADDRESS_SIZE,
        BALANCE_SIZE,
    )
}

// TODO: make this private.
/// A state for maintaining a stable block that is partially ingested into the UTXO set.
/// Used for time slicing.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq)]
pub struct PartialStableBlock {
    pub block: Block,
    pub next_tx_idx: usize,
    pub next_input_idx: usize,
    pub next_output_idx: usize,
    pub stats: BlockIngestionStats,
}

impl PartialStableBlock {
    pub fn new(
        block: Block,
        next_tx_idx: usize,
        next_input_idx: usize,
        next_output_idx: usize,
    ) -> Self {
        Self {
            block,
            next_tx_idx,
            next_input_idx,
            next_output_idx,
            stats: BlockIngestionStats::default(),
        }
    }
}

// NOTE: `PartialEq` is only available in tests as it would be impractically
// expensive in production.
#[cfg(test)]
impl PartialEq for UtxoSet {
    fn eq(&self, other: &Self) -> bool {
        use crate::test_utils::is_stable_btreemap_equal;
        self.utxos == other.utxos
            && self.network == other.network
            && self.next_height == other.next_height
            && self.partial_stable_block == other.partial_stable_block
            && is_stable_btreemap_equal(&self.address_to_outpoints, &other.address_to_outpoints)
            && is_stable_btreemap_equal(&self.balances, &other.balances)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{random_p2pkh_address, TransactionBuilder};
    use crate::{
        address_utxoset::AddressUtxoSet,
        types::{Network, OutPoint, Txid},
        unstable_blocks::UnstableBlocks,
    };
    use bitcoin::blockdata::{opcodes::all::OP_RETURN, script::Builder};
    use ic_btc_types::Height;
    use std::collections::BTreeSet;

    // A succinct wrapper around `ingest_tx_with_slicing` for tests that don't need slicing.
    fn ingest_tx(utxo_set: &mut UtxoSet, tx: &Transaction) {
        assert_eq!(
            utxo_set.ingest_tx_with_slicing(tx, 0, 0, &mut BlockIngestionStats::default()),
            Slicing::Done
        );
    }

    #[test]
    fn tx_without_outputs_leaves_utxo_set_unchanged() {
        for network in [Network::Mainnet, Network::Regtest, Network::Testnet].iter() {
            let mut utxo = UtxoSet::new(*network);

            // no output coinbase
            let coinbase_empty_tx = Transaction::new(bitcoin::Transaction {
                output: vec![],
                input: vec![],
                version: 1,
                lock_time: 0,
            });
            ingest_tx(&mut utxo, &coinbase_empty_tx);

            assert!(utxo.utxos.is_empty());
            assert!(utxo.address_to_outpoints.is_empty());
        }
    }

    #[test]
    fn filter_provably_unspendable_utxos() {
        for network in [Network::Mainnet, Network::Regtest, Network::Testnet].iter() {
            let mut utxo = UtxoSet::new(*network);

            // op return coinbase
            let coinbase_op_return_tx = Transaction::new(bitcoin::Transaction {
                output: vec![BitcoinTxOut {
                    value: 50_0000_0000,
                    script_pubkey: Builder::new().push_opcode(OP_RETURN).into_script(),
                }],
                input: vec![],
                version: 1,
                lock_time: 0,
            });
            ingest_tx(&mut utxo, &coinbase_op_return_tx);

            assert!(utxo.utxos.is_empty());
            assert!(utxo.address_to_outpoints.is_empty());
        }
    }

    #[test]
    fn spending_mainnet() {
        spending(Network::Mainnet);
    }

    #[test]
    fn spending_testnet() {
        spending(Network::Testnet);
    }

    #[test]
    fn spending_regtest() {
        spending(Network::Regtest);
    }

    fn spending(network: Network) {
        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);

        let mut utxo = UtxoSet::new(network);

        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        ingest_tx(&mut utxo, &coinbase_tx);

        let unstable_blocks = UnstableBlocks::new(&utxo, 2, crate::genesis_block(network));

        let expected = vec![ic_btc_types::Utxo {
            outpoint: ic_btc_types::OutPoint {
                txid: coinbase_tx.txid().to_vec(),
                vout: 0,
            },
            value: 1000,
            height: 0,
        }];

        assert_eq!(
            AddressUtxoSet::new(address_1.clone(), &utxo, &unstable_blocks).into_vec(None),
            expected
        );
        assert_eq!(
            utxo.address_to_outpoints
                .iter()
                .map(|(k, _)| <(Address, Height, OutPoint)>::from_bytes(k))
                .collect::<BTreeSet<_>>(),
            maplit::btreeset! {
                (address_1.clone(), 0, OutPoint::new(coinbase_tx.txid(), 0))
            }
        );

        utxo.next_height += 1;

        // Spend the output to address 2.
        let tx = TransactionBuilder::new()
            .with_input(OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_2, 1000)
            .build();
        ingest_tx(&mut utxo, &tx);

        assert_eq!(
            AddressUtxoSet::new(address_1, &utxo, &unstable_blocks).into_vec(None),
            vec![]
        );
        assert_eq!(
            AddressUtxoSet::new(address_2.clone(), &utxo, &unstable_blocks).into_vec(None),
            vec![ic_btc_types::Utxo {
                outpoint: ic_btc_types::OutPoint {
                    txid: tx.txid().to_vec(),
                    vout: 0
                },
                value: 1000,
                height: 1
            }]
        );
        assert_eq!(
            utxo.address_to_outpoints
                .iter()
                .map(|(k, _)| <(Address, Height, OutPoint)>::from_bytes(k))
                .collect::<BTreeSet<_>>(),
            maplit::btreeset! {
                (address_2, 1, OutPoint::new(tx.txid(), 0))
            }
        );
    }

    #[test]
    fn utxos_are_sorted_by_height() {
        let address = random_p2pkh_address(Network::Testnet);

        let mut utxo = UtxoSet::new(Network::Testnet);

        // Insert some entries into the map with different heights in some random order.
        for height in [17u32, 0, 31, 4, 2].iter() {
            utxo.address_to_outpoints
                .insert(
                    (
                        address.clone(),
                        *height,
                        OutPoint::new(Txid::from(vec![0; 32]), 0),
                    )
                        .to_bytes(),
                    vec![],
                )
                .unwrap();
        }

        // Verify that the entries returned are sorted in descending height.
        assert_eq!(
            utxo.address_to_outpoints
                .range(address.to_bytes().to_vec(), None)
                .map(|(k, _)| {
                    let (_, height, _) = <(Address, Height, OutPoint)>::from_bytes(k);
                    height
                })
                .collect::<Vec<_>>(),
            vec![31, 17, 4, 2, 0]
        );
    }

    #[test]
    #[should_panic]
    fn inserting_same_outpoint_panics() {
        let network = Network::Testnet;
        let mut utxo_set = UtxoSet::new(network);
        let address = random_p2pkh_address(network);

        let tx_out_1 = TransactionBuilder::coinbase()
            .with_output(&address, 1000)
            .build()
            .output()[0]
            .clone();

        let tx_out_2 = TransactionBuilder::coinbase()
            .with_output(&address, 2000)
            .build()
            .output()[0]
            .clone();

        let outpoint = OutPoint::new(Txid::from(vec![]), 0);

        utxo_set.insert_utxo(outpoint.clone(), tx_out_1);

        // Should panic, as we are trying to insert a UTXO with the same outpoint.
        utxo_set.insert_utxo(outpoint, tx_out_2);
    }

    #[test]
    fn addresses_with_empty_balances_are_removed() {
        let network = Network::Testnet;
        let mut utxo_set = UtxoSet::new(network);
        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);

        let tx_1 = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();

        let tx_2 = TransactionBuilder::new()
            .with_input(OutPoint {
                txid: tx_1.txid(),
                vout: 0,
            })
            .with_output(&address_2, 1000)
            .build();

        // Ingesting the first transaction. There should be one entry in the balance
        // map containing address 1.
        ingest_tx(&mut utxo_set, &tx_1);
        assert_eq!(utxo_set.balances.len(), 1);
        assert_eq!(utxo_set.balances.get(&address_1), Some(1000));

        // Ingesting the second transaction. There should be one entry in the balance
        // map containing address 2. Address 1 should be removed as it's balance is zero.
        ingest_tx(&mut utxo_set, &tx_2);
        assert_eq!(utxo_set.balances.len(), 1);
        assert_eq!(utxo_set.balances.get(&address_2), Some(1000));
    }
}
