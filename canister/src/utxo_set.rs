use crate::{
    memory::Memory,
    multi_iter::MultiIter,
    runtime::{inc_performance_counter, performance_counter, print},
    types::{
        Address, AddressUtxo, Block, BlockHash, Network, OutPoint, Slicing, Storable, Transaction,
        TxOut, Txid, Utxo,
    },
};
use bitcoin::{Script, TxOut as BitcoinTxOut};
use ic_btc_types::{Height, Satoshi};
use ic_stable_structures::{StableBTreeMap, Storable as _};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, iter::Iterator, str::FromStr};
mod utxos;
mod utxos_delta;
use utxos::Utxos;
use utxos_delta::UtxosDelta;

lazy_static::lazy_static! {
    pub static ref DUPLICATE_TX_IDS: [Txid; 2] = [
        Txid::from_str("d5d27987d2a3dfc724e359870c6644b40e497bdc0589a033220fe15429d88599").unwrap(),
        Txid::from_str("e3bf3d07d4b0375638d5f1db5255fe07ba2c4cb067cd81b84ee974b6585fb468").unwrap(),
    ];
}

#[derive(Serialize, Deserialize)]
pub struct UtxoSet {
    pub utxos: Utxos,

    network: Network,

    // An index for fast retrievals of an address's UTXOs.
    // NOTE: Stable structures don't need to be serialized.
    #[serde(skip, default = "init_address_utxos")]
    address_utxos: StableBTreeMap<Memory, AddressUtxo, ()>,

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
    pub next_height: Height,

    // The predicate used to determine whether or not we should time-slice.
    // The default predicate is to check the performance counter, but can be overridden for tests.
    #[serde(skip, default = "default_should_time_slice")]
    should_time_slice: Box<dyn FnMut() -> bool>,

    /// A block that is currently being ingested into the UtxoSet. Used for time slicing.
    pub ingesting_block: Option<IngestingBlock>,
}

impl UtxoSet {
    pub fn new(network: Network) -> Self {
        Self {
            utxos: Utxos::default(),
            balances: init_balances(),
            address_utxos: init_address_utxos(),
            network,
            next_height: 0,
            ingesting_block: None,
            should_time_slice: default_should_time_slice(),
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
    pub fn ingest_block(&mut self, block: Block) -> Slicing<(), BlockHash> {
        assert!(
            self.ingesting_block.is_none(),
            "Cannot ingest new block while previous block (height {}) isn't fully ingested",
            self.next_height
        );

        // Store in the state the new block to be ingested.
        self.ingesting_block = Some(IngestingBlock::new(block));

        // Start ingesting.
        self.ingest_block_continue()
            .expect("a block to ingest must exist.")
    }

    /// Continue ingesting a block.
    /// Returns:
    ///   * `None` if there was no block to continue ingesting.
    ///   * `Slicing::Done(block_hash)` if the partially ingested block is now fully ingested,
    ///      where `block_hash` is the hash of the ingested block.
    ///   * `Slicing::Paused(())` if the block continued to be ingested, but is time-sliced.
    pub fn ingest_block_continue(&mut self) -> Option<Slicing<(), BlockHash>> {
        let ins_start = performance_counter();

        let IngestingBlock {
            block,
            next_tx_idx,
            mut next_input_idx,
            mut next_output_idx,
            mut utxos_delta,
            mut stats,
        } = match self.ingesting_block.take() {
            Some(p) => p,
            None => return None,
        };

        stats.num_rounds += 1;
        for (tx_idx, tx) in block.txdata().iter().enumerate().skip(next_tx_idx) {
            if let Slicing::Paused((next_input_idx, next_output_idx)) = self.ingest_tx_with_slicing(
                tx,
                next_input_idx,
                next_output_idx,
                &mut utxos_delta,
                &mut stats,
            ) {
                stats.ins_total += performance_counter() - ins_start;

                // Getting close to the the instructions limit. Pause execution.
                self.ingesting_block = Some(IngestingBlock {
                    block,
                    next_tx_idx: tx_idx,
                    next_input_idx,
                    next_output_idx,
                    utxos_delta,
                    stats,
                });

                return Some(Slicing::Paused(()));
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
        Some(Slicing::Done(block.block_hash()))
    }

    /// Returns the balance of the given address.
    pub fn get_balance(&self, address: &Address) -> Satoshi {
        let mut balance = self.balances.get(address).unwrap_or(0);

        // Revert any changes to the balance that were done by the ingesting block.
        if let Some(ingesting_block) = &self.ingesting_block {
            let utxos_delta = &ingesting_block.utxos_delta;

            // Add any removed outpoints back to the balance.
            for outpoint in utxos_delta.get_removed_outpoints(address) {
                let (tx_out, _) = utxos_delta.get_utxo(outpoint).expect("UTXO must exist");
                balance = balance.checked_add(tx_out.value).expect("Cannot overflow");
            }

            // Remove any added outpoints from the balance.
            for outpoint in utxos_delta.get_added_outpoints(address) {
                let (tx_out, _) = utxos_delta.get_utxo(outpoint).expect("UTXO must exist");
                balance = balance.checked_sub(tx_out.value).expect("Cannot underflow");
            }
        }

        balance
    }

    /// Returns the UTXO of the given outpoint.
    pub fn get_utxo(&self, outpoint: &OutPoint) -> Option<(TxOut, Height)> {
        // Revert any changes to the UTXOs that were done by the ingesting block.
        if let Some(b) = &self.ingesting_block {
            if b.utxos_delta.is_outpoint_removed(outpoint) {
                // The UTXO was removed by the ingesting block.
                // Revert that removal by returning the UTXO.
                return b.utxos_delta.get_utxo(outpoint).cloned();
            }

            if b.utxos_delta.is_outpoint_added(outpoint) {
                // The UTXO was added by the ingesting block.
                // Revert that addition by returning `None`.
                return None;
            }
        };

        // No modifications done by the ingesting block. Return the UTXO from the stable set.
        self.utxos.get(outpoint)
    }

    /// Returns an iterator with the outpoints of the given address.
    /// An optional offset can be specified for pagination.
    pub fn get_address_outpoints(
        &self,
        address: &Address,
        offset: &Option<Utxo>,
    ) -> impl Iterator<Item = OutPoint> + '_ {
        // If there is an ingesting block, retrieve all the outpoints it added/removed.
        let (added_outpoints, removed_outpoints) = match &self.ingesting_block {
            Some(b) => (
                b.utxos_delta.get_added_outpoints(address),
                b.utxos_delta.get_removed_outpoints(address),
            ),
            None => (BTreeSet::new(), BTreeSet::new()),
        };

        // Retrieve all address's outpoints from the stable set, removing any outpoints
        // that were added by the ingesting block.
        let stable_outpoints = self
            .address_utxos
            .range(
                address.to_bytes().to_vec(),
                offset
                    .as_ref()
                    .map(|u| (u.height, u.outpoint.clone()).to_bytes()),
            )
            .map(|(address_utxo, _)| address_utxo.outpoint)
            .filter(move |outpoint| !added_outpoints.contains(outpoint));

        // Return the stable outpoints along with the outpoints removed by the ingesting block.
        MultiIter::new(stable_outpoints, removed_outpoints.into_iter().cloned())
    }

    /// Returns the number of UTXOs in the set.
    pub fn utxos_len(&self) -> u64 {
        self.utxos.len()
    }

    /// Returns the number of UTXOs that are owned by supported addresses.
    pub fn address_utxos_len(&self) -> u64 {
        self.address_utxos.len()
    }

    /// Returns the number of addresses that we have balances for.
    pub fn balances_len(&self) -> u64 {
        self.balances.len()
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub fn next_height(&self) -> Height {
        self.next_height
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
        utxos_delta: &mut UtxosDelta,
        stats: &mut BlockIngestionStats,
    ) -> Slicing<(usize, usize), ()> {
        let ins_start = performance_counter();
        let res = self.remove_inputs(tx, start_input_idx, utxos_delta);
        stats.ins_remove_inputs += performance_counter() - ins_start;
        if let Slicing::Paused(input_idx) = res {
            return Slicing::Paused((input_idx, 0));
        }

        let ins_start = performance_counter();
        let res = self.insert_outputs(tx, start_output_idx, utxos_delta, stats);
        stats.ins_insert_outputs += performance_counter() - ins_start;
        if let Slicing::Paused(output_idx) = res {
            return Slicing::Paused((tx.input().len(), output_idx));
        }

        Slicing::Done(())
    }

    // Iterates over transaction inputs, starting from `start_idx`, and removes them from the UTXO set.
    fn remove_inputs(
        &mut self,
        tx: &Transaction,
        start_idx: usize,
        utxos_delta: &mut UtxosDelta,
    ) -> Slicing<usize, ()> {
        if tx.is_coin_base() {
            return Slicing::Done(());
        }

        for (input_idx, input) in tx.input().iter().enumerate().skip(start_idx) {
            if (self.should_time_slice)() {
                return Slicing::Paused(input_idx);
            }

            // Remove the input from the UTXOs. The input *must* exist in the UTXO set.
            let outpoint = (&input.previous_output).into();
            match self.utxos.remove(&outpoint) {
                Some((txout, height)) => {
                    if let Ok(address) = Address::from_script(
                        &Script::from(txout.script_pubkey.clone()),
                        self.network,
                    ) {
                        let address_utxo = AddressUtxo {
                            address: address.clone(),
                            height,
                            outpoint: outpoint.clone(),
                        };

                        let found = self.address_utxos.remove(&address_utxo);

                        assert!(
                            found.is_some(),
                            "Outpoint {:?} not found in the index.",
                            input.previous_output
                        );

                        // Update the balance of the address.
                        if txout.value != 0 {
                            let address_balance =
                                self.balances.get(&address).unwrap_or_else(|| {
                                    panic!("Address {} must exist in the balances map (trying to remove outpoint {:?})", address, input.previous_output);
                                });

                            match address_balance - txout.value {
                                // Remove the address from the map if balance is zero.
                                0 => self.balances.remove(&address),
                                // Update the balance in the map.
                                balance => self.balances.insert(address.clone(), balance).unwrap(),
                            };
                        }

                        utxos_delta.remove(address, outpoint, txout, height);
                    }
                }
                None => {
                    panic!("Outpoint {:?} not found.", outpoint);
                }
            }
        }

        Slicing::Done(())
    }

    // Iterates over transaction outputs, starting from `start_idx`, and inserts them into the UTXO set.
    fn insert_outputs(
        &mut self,
        tx: &Transaction,
        start_idx: usize,
        utxos_delta: &mut UtxosDelta,
        stats: &mut BlockIngestionStats,
    ) -> Slicing<usize, ()> {
        for (vout, output) in tx.output().iter().enumerate().skip(start_idx) {
            if (self.should_time_slice)() {
                return Slicing::Paused(vout);
            }

            if !(output.script_pubkey.is_provably_unspendable()) {
                let ins_start = performance_counter();
                let txid = tx.txid();
                stats.ins_txids += performance_counter() - ins_start;

                let ins_start = performance_counter();
                self.insert_utxo(
                    OutPoint::new(txid, vout as u32),
                    output.clone(),
                    utxos_delta,
                );
                stats.ins_insert_utxos += performance_counter() - ins_start;
            }
        }

        Slicing::Done(())
    }

    // Inserts a UTXO into the given UTXO set.
    // A UTXO is represented by the the tuple: (outpoint, output)
    fn insert_utxo(
        &mut self,
        outpoint: OutPoint,
        output: BitcoinTxOut,
        utxos_delta: &mut UtxosDelta,
    ) {
        // Insert the outpoint.
        let tx_out: TxOut = (&output).into();
        if let Ok(address) = Address::from_script(&output.script_pubkey, self.network) {
            // Add the address to the index if we can parse it.
            self.address_utxos
                .insert(
                    AddressUtxo {
                        address: address.clone(),
                        height: self.next_height,
                        outpoint: outpoint.clone(),
                    },
                    (),
                )
                .expect("insertion must succeed");

            // Update the balance of the address.
            let address_balance = self.balances.get(&address).unwrap_or(0);
            self.balances
                .insert(address.clone(), address_balance + output.value)
                .expect("insertion must succeed");

            utxos_delta.insert(address, outpoint.clone(), tx_out.clone(), self.next_height);
        }

        let outpoint_already_exists = self
            .utxos
            .insert(outpoint.clone(), (tx_out, self.next_height));

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

fn init_address_utxos() -> StableBTreeMap<Memory, AddressUtxo, ()> {
    StableBTreeMap::init(crate::memory::get_address_utxos_memory())
}

fn init_balances() -> StableBTreeMap<Memory, Address, u64> {
    StableBTreeMap::init(crate::memory::get_balances_memory())
}

/// A state for maintaining a stable block that is partially ingested into the UTXO set.
/// Used for time slicing.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq)]
pub struct IngestingBlock {
    pub block: Block,
    pub next_tx_idx: usize,
    pub next_input_idx: usize,
    pub next_output_idx: usize,
    stats: BlockIngestionStats,
    utxos_delta: UtxosDelta,
}

impl IngestingBlock {
    pub fn new(block: Block) -> Self {
        Self {
            block,
            next_tx_idx: 0,
            next_input_idx: 0,
            next_output_idx: 0,
            stats: BlockIngestionStats::default(),
            utxos_delta: UtxosDelta::default(),
        }
    }

    #[cfg(test)]
    pub fn new_with_args(
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
            utxos_delta: UtxosDelta::default(),
        }
    }
}

// Various profiling stats for tracking the performance of block ingestion.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq, Default)]
struct BlockIngestionStats {
    // The number of rounds it took to ingest the block.
    num_rounds: u32,

    // The total number of instructions used to ingest the block.
    ins_total: u64,

    // The number of instructions used to remove the transaction inputs.
    ins_remove_inputs: u64,

    // The number of instructions used to insert the transaction outputs.
    ins_insert_outputs: u64,

    // The number of instructions used to compute the txids.
    ins_txids: u64,

    // The number of instructions used to insert new utxos.
    ins_insert_utxos: u64,
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
            && self.ingesting_block == other.ingesting_block
            && is_stable_btreemap_equal(&self.address_utxos, &other.address_utxos)
            && is_stable_btreemap_equal(&self.balances, &other.balances)
    }
}

// The default predicate to use for time-slicing.
// Checks that we're not approaching the instructions limit.
fn default_should_time_slice() -> Box<dyn FnMut() -> bool> {
    // The threshold at which time slicing kicks in.
    // At the time of this writing it is equivalent to 80% of the maximum instructions limit.
    const MAX_INSTRUCTIONS_THRESHOLD: u64 = 4_000_000_000;

    // NOTE: We're using `inc_performance_counter` here to also increment the mock performance
    // counter in the unit tests.
    Box::new(|| inc_performance_counter() >= MAX_INSTRUCTIONS_THRESHOLD)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{random_p2pkh_address, BlockBuilder, TransactionBuilder};
    use crate::{
        address_utxoset::AddressUtxoSet,
        types::{Network, OutPoint, Txid},
        unstable_blocks::UnstableBlocks,
    };
    use bitcoin::blockdata::{opcodes::all::OP_RETURN, script::Builder};
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    // A succinct wrapper around `ingest_tx_with_slicing` for tests that don't need slicing.
    fn ingest_tx(utxo_set: &mut UtxoSet, tx: &Transaction) {
        assert_eq!(
            utxo_set.ingest_tx_with_slicing(
                tx,
                0,
                0,
                &mut UtxosDelta::default(),
                &mut BlockIngestionStats::default()
            ),
            Slicing::Done(())
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
            assert!(utxo.address_utxos.is_empty());
        }
    }

    #[test]
    fn filter_provably_unspendable_utxos() {
        for network in [Network::Mainnet, Network::Regtest, Network::Testnet].iter() {
            let mut utxo = UtxoSet::new(*network);

            // A provably unspendable tx.
            let block = BlockBuilder::genesis()
                .with_transaction(Transaction::new(bitcoin::Transaction {
                    output: vec![BitcoinTxOut {
                        value: 50_0000_0000,
                        script_pubkey: Builder::new().push_opcode(OP_RETURN).into_script(),
                    }],
                    input: vec![],
                    version: 1,
                    lock_time: 0,
                }))
                .build();

            assert_eq!(
                utxo.ingest_block(block.clone()),
                Slicing::Done(block.block_hash())
            );
            assert!(utxo.utxos.is_empty());
            assert!(utxo.address_utxos.is_empty());
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

        let expected = vec![Utxo {
            outpoint: OutPoint {
                txid: coinbase_tx.txid(),
                vout: 0,
            },
            value: 1000,
            height: 0,
        }];

        assert_eq!(
            AddressUtxoSet::new(address_1.clone(), &utxo, &unstable_blocks)
                .into_iter(None)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            utxo.address_utxos
                .iter()
                .map(|(k, _)| k)
                .collect::<BTreeSet<_>>(),
            maplit::btreeset! {
                AddressUtxo {
                    address: address_1.clone(),
                    height: 0,
                    outpoint: OutPoint::new(coinbase_tx.txid(), 0)
                }
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
            AddressUtxoSet::new(address_1, &utxo, &unstable_blocks)
                .into_iter(None)
                .collect::<Vec<_>>(),
            vec![]
        );
        assert_eq!(
            AddressUtxoSet::new(address_2.clone(), &utxo, &unstable_blocks)
                .into_iter(None)
                .collect::<Vec<_>>(),
            vec![Utxo {
                outpoint: OutPoint {
                    txid: tx.txid(),
                    vout: 0
                },
                value: 1000,
                height: 1
            }]
        );
        assert_eq!(
            utxo.address_utxos
                .iter()
                .map(|(k, _)| k)
                .collect::<BTreeSet<_>>(),
            maplit::btreeset! {
                AddressUtxo {
                    address: address_2,
                    height: 1,
                    outpoint: OutPoint::new(tx.txid(), 0)
                }
            }
        );
    }

    #[test]
    fn utxos_are_sorted_by_height() {
        let address = random_p2pkh_address(Network::Testnet);

        let mut utxo = UtxoSet::new(Network::Testnet);

        // Insert some entries into the map with different heights in some random order.
        for height in [17u32, 0, 31, 4, 2].iter() {
            utxo.address_utxos
                .insert(
                    AddressUtxo {
                        address: address.clone(),
                        height: *height,
                        outpoint: OutPoint::new(Txid::from(vec![0; 32]), 0),
                    },
                    (),
                )
                .unwrap();
        }

        // Verify that the entries returned are sorted in descending height.
        assert_eq!(
            utxo.address_utxos
                .range(address.to_bytes().to_vec(), None)
                .map(|(address_utxo, _)| { address_utxo.height })
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

        utxo_set.insert_utxo(outpoint.clone(), tx_out_1, &mut UtxosDelta::default());

        // Should panic, as we are trying to insert a UTXO with the same outpoint.
        utxo_set.insert_utxo(outpoint, tx_out_2, &mut UtxosDelta::default());
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

    // An edge case where an address has a UTXO with zero value. The address starts with a
    // positive balance, then all positive UTXOs are consumed, then the UTXO with zero value
    // is consumed.
    // This cannot happen on mainnet, but can and has happened on testnet.
    #[test]
    fn consuming_an_input_with_value_zero() {
        let network = Network::Testnet;
        let mut utxo_set = UtxoSet::new(network);
        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);

        let tx_1 = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .with_output(&address_1, 0) // an input with zero value
            .build();

        // Consume the first input in tx 1
        let tx_2 = TransactionBuilder::new()
            // Consume the positive UTXO
            .with_input(OutPoint {
                txid: tx_1.txid(),
                vout: 0,
            })
            // then consume the zero UTXO
            .with_input(OutPoint {
                txid: tx_1.txid(),
                vout: 1,
            })
            .with_output(&address_2, 1000)
            .build();

        let block = BlockBuilder::genesis()
            .with_transaction(tx_1)
            .with_transaction(tx_2)
            .build();

        assert_eq!(
            utxo_set.ingest_block(block.clone()),
            Slicing::Done(block.block_hash())
        );
        assert_eq!(utxo_set.get_balance(&address_1), 0);
        assert_eq!(utxo_set.get_balance(&address_2), 1_000);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]
        #[test]
        fn atomicity_while_ingesting_block(
            // The number of inputs/outputs to ingest per round.
            ingestion_rate in 1..150u32,

            // The number of inputs/outputs per transaction.
            tx_cardinality in 1..200u64,

            network in prop_oneof![
                Just(Network::Mainnet),
                Just(Network::Testnet),
                Just(Network::Regtest),
            ]) {

            let address_1 = random_p2pkh_address(network);
            let address_2 = random_p2pkh_address(network);
            let address_3 = random_p2pkh_address(network);

            let mut utxo_set = UtxoSet::new(network);

            // Transaction 0: A coinbase tx with `tx_cardinality` inputs, each giving 1 Satoshi to
            // address 1.
            let mut tx_0 = TransactionBuilder::coinbase();
            for i in 0..tx_cardinality {
                tx_0 = tx_0.with_output(&address_1, 1).with_lock_time(i as u32)
            }
            let tx_0 = tx_0.build();

            // Transaction 1: Consume all the outputs of transaction 0 *in reverse order* and create
            // similar outputs for address 2.
            //
            // Consuming the inputs in reverse order here is deliberate as it allows to test
            // whether or not they'll be sorted when they're re-added back in
            // `get_address_outpoints`.
            let mut tx_1 = TransactionBuilder::new();
            for i in (0..tx_cardinality).rev() {
                tx_1 = tx_1.with_input(OutPoint {
                    vout: i as u32,
                    txid: tx_0.txid(),
                });
            }
            for i in 0..tx_cardinality {
                tx_1 = tx_1.with_output(&address_2, 1).with_lock_time(i as u32);
            }
            let tx_1 = tx_1.build();

            // Transaction 2: Consume all the outputs of transaction 1 and create similar outputs
            // for address 3.
            let mut tx_2 = TransactionBuilder::new();
            for i in 0..tx_cardinality {
                tx_2 = tx_2
                    .with_input(OutPoint {
                        vout: i as u32,
                        txid: tx_1.txid(),
                    })
                    .with_output(&address_3, 1);
            }
            let tx_2 = tx_2.build();

            // Block 0: Contains transaction 0.
            let block_0 = BlockBuilder::genesis()
                .with_transaction(tx_0.clone())
                .build();

            // Block 1: Contains transactions 1 and 2.
            let block_1 = BlockBuilder::with_prev_header(block_0.header())
                .with_transaction(tx_1.clone())
                .with_transaction(tx_2.clone())
                .build();


            // Ingest block 0 without any time-slicing.
            assert_eq!(
                utxo_set.ingest_block(block_0.clone()),
                Slicing::Done(block_0.block_hash())
            );

            // Update predicate to time-slice block 1 based on the ingestion rate.
            utxo_set.should_time_slice = ingestion_rate_predicate(ingestion_rate);

            let res = utxo_set.ingest_block(block_1);
            let mut num_rounds = 1;

            if Slicing::Paused(()) == res {
                num_rounds += 1;
                while let Some(Slicing::Paused(())) = utxo_set.ingest_block_continue() {

                    // Block 1 ingestion is paused. Assert that the state is exactly
                    // what we expect if only block 0 is ingested.

                    assert_eq!(utxo_set.get_balance(&address_1), tx_cardinality);
                    assert_eq!(utxo_set.get_balance(&address_2), 0);
                    assert_eq!(utxo_set.get_balance(&address_3), 0);

                    assert_eq!(
                        utxo_set
                            .get_address_outpoints(&address_1, &None)
                            .collect::<Vec<_>>(),
                        (0..tx_cardinality)
                            .map(|i| OutPoint {
                                txid: tx_0.txid(),
                                vout: i as u32
                            })
                            .collect::<Vec<_>>()
                    );

                    assert_eq!(
                        utxo_set
                            .get_address_outpoints(&address_2, &None)
                            .collect::<Vec<_>>(),
                        vec![]
                    );

                    assert_eq!(
                        utxo_set
                            .get_address_outpoints(&address_3, &None)
                            .collect::<Vec<_>>(),
                        vec![]
                    );

                    for i in 0..tx_cardinality {
                        // All the outpoints in block 0 exist.
                        assert!(utxo_set
                            .get_utxo(&OutPoint {
                                vout: i as u32,
                                txid: tx_0.txid(),
                            })
                            .is_some());

                        // All the outpoints in block 1 do not exist.
                        assert_eq!(
                            utxo_set.get_utxo(&OutPoint {
                                vout: i as u32,
                                txid: tx_1.txid(),
                            }),
                            None
                        );

                        assert_eq!(
                            utxo_set.get_utxo(&OutPoint {
                                vout: i as u32,
                                txid: tx_2.txid(),
                            }),
                            None
                        );
                    }

                    num_rounds += 1;
                }
            }

            // Finished ingesting block 1. Assert that the balances, addresses outpoints, and
            // UTXOs are updated accordingly.
            assert_eq!(utxo_set.get_balance(&address_1), 0);
            assert_eq!(utxo_set.get_balance(&address_2), 0);
            assert_eq!(utxo_set.get_balance(&address_3), tx_cardinality);
            assert_eq!(
                num_rounds,
                ((tx_cardinality * 4) as f32 / ingestion_rate as f32).ceil() as u32
            );

            assert_eq!(
                utxo_set
                    .get_address_outpoints(&address_1, &None)
                    .collect::<Vec<_>>(),
                vec![]
            );

            assert_eq!(
                utxo_set
                    .get_address_outpoints(&address_2, &None)
                    .collect::<Vec<_>>(),
                vec![]
            );

            assert_eq!(
                utxo_set
                    .get_address_outpoints(&address_3, &None)
                    .collect::<Vec<_>>(),
                (0..tx_cardinality)
                    .map(|i| OutPoint {
                        txid: tx_2.txid(),
                        vout: i as u32
                    })
                    .collect::<Vec<_>>()
            );

            for i in 0..tx_cardinality {
                // All the outpoints in tx 0 don't exist.
                assert_eq!(
                    utxo_set.get_utxo(&OutPoint {
                        vout: i as u32,
                        txid: tx_0.txid(),
                    }),
                    None
                );

                // All the outpoints in tx 1 don't exist.
                assert_eq!(
                    utxo_set.get_utxo(&OutPoint {
                        vout: i as u32,
                        txid: tx_1.txid(),
                    }),
                    None
                );

                // All the outpoints in tx 2 exist.
                assert!(utxo_set
                    .get_utxo(&OutPoint {
                        vout: i as u32,
                        txid: tx_2.txid(),
                    }).is_some());
            }
        }
    }

    // A predicate that allows the Utxo Set to ingest `ingestion_rate` inputs/outputs,
    // then triggers time-slicing.
    fn ingestion_rate_predicate(ingestion_rate: u32) -> Box<dyn FnMut() -> bool> {
        let mut count = ingestion_rate + 1;
        Box::new(move || {
            count -= 1;
            if count == 0 {
                // Trigger time-slicing, but reset the counter before doing so.
                count = ingestion_rate + 1;
                true
            } else {
                false
            }
        })
    }
}
