use crate::{
    memory::Memory,
    runtime::{inc_performance_counter, performance_counter, print},
    state::{BlockIngestionStats, OUTPOINT_SIZE},
    types::{
        Address, AddressUtxo, Block, Network, OutPoint, Slicing, Storable, Transaction, TxOut, Txid,
    },
};
use bitcoin::{BlockHash, Script, TxOut as BitcoinTxOut};
use ic_btc_types::{Height, Satoshi};
use ic_stable_structures::{btreemap::Iter, StableBTreeMap, Storable as _};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};
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
    next_height: Height,

    /// A stable block that has partially been written to the UTXO set. Used for time slicing.
    pub partial_stable_block: Option<PartialStableBlock>,
}

impl UtxoSet {
    pub fn new(network: Network) -> Self {
        Self {
            utxos: Utxos::default(),
            balances: init_balances(),
            address_utxos: init_address_utxos(),
            network,
            next_height: 0,
            partial_stable_block: None,
        }
    }

    /// Ingests a block into the `UtxoSet`.
    ///
    /// The inputs of all the transactions in the block are removed and the outputs are inserted.
    /// The block is assumed to be valid, so a failure of any of these operations causes a panic.
    ///
    /// Returns `Slicing::Done` with the block hack of the ingested block if ingestion is complete,
    /// or `Slicing::Paused` if ingestion hasn't fully completed due to instruction limits. In the
    /// latter case, one or more calls to `ingest_block_continue` are necessary to finish the block
    /// ingestion.
    pub fn ingest_block(&mut self, block: Block) -> Slicing<(), BlockHash> {
        assert!(
            self.partial_stable_block.is_none(),
            "Cannot ingest new block while previous block (height {}) isn't fully ingested",
            self.next_height
        );

        self.ingest_block_helper(
            block,
            0,
            0,
            0,
            UtxosDelta::default(),
            BlockIngestionStats::default(),
        )
    }

    /// Continue ingesting a block.
    ///
    /// Returns:
    ///   * `Slicing::Paused` if ingested hasn't fully completed due to instruction limit.
    ///   * `Slicing::Done(None)` if there was no block to continue ingesting.
    ///   * `Slicing::Done(Some(block_hash))` if there was a block to continue ingesting.
    ///      The block hash of the ingested block is returned.
    pub fn ingest_block_continue(&mut self) -> Slicing<(), Option<BlockHash>> {
        match self.partial_stable_block.take() {
            Some(p) => {
                let res = self.ingest_block_helper(
                    p.block,
                    p.next_tx_idx,
                    p.next_input_idx,
                    p.next_output_idx,
                    p.utxos_delta,
                    p.stats,
                );

                match res {
                    Slicing::Done(block_hash) => Slicing::Done(Some(block_hash)),
                    Slicing::Paused(e) => Slicing::Paused(e),
                }
            }
            None => {
                // No partially ingested block found. Nothing to do.
                Slicing::Done(None)
            }
        }
    }

    /// Returns the balance of the given address.
    pub fn get_balance(&self, address: &Address) -> Satoshi {
        let mut stable_balance = self.balances.get(address).unwrap_or(0);

        // Revert any changes to the stable balance that were done by the block being currently
        // ingested.
        if let Some(p) = &self.partial_stable_block {
            // Add any removed outpoints back to the balance.
            if let Some(removed_outpoints) = p.utxos_delta.removed_outpoints.get(address) {
                for outpoint in removed_outpoints {
                    let (tx_out, _) = p.utxos_delta.utxos.get(outpoint).expect("UTXO must exist");
                    stable_balance += tx_out.value;
                }
            }

            // Remove any added outpoints from the balance.
            if let Some(added_outpoints) = p.utxos_delta.added_outpoints.get(address) {
                for outpoint in added_outpoints {
                    let (tx_out, _) = p.utxos_delta.utxos.get(outpoint).expect("UTXO must exist");
                    stable_balance -= tx_out.value;
                }
            }
        }

        stable_balance
    }

    /// Returns the UTXO of the given outpoint.
    pub fn get_utxo(&self, outpoint: &OutPoint) -> Option<(TxOut, Height)> {
        // TODO(EXC-1231): Revert any changes to the stable balance that were done by the block
        // being currently ingested.
        self.utxos.get(outpoint)
    }

    /// Returns the outpoints owned by the given address.
    /// An optional offset can be specified for pagination.
    pub fn get_address_utxos(
        &self,
        address: &Address,
        offset: &Option<(Height, OutPoint)>,
    ) -> Iter<Memory, AddressUtxo, ()> {
        self.address_utxos.range(
            address.to_bytes().to_vec(),
            offset.as_ref().map(|x| x.to_bytes()),
        )
    }

    /// Returns the number of UTXOs in the set.
    pub fn utxos_len(&self) -> u64 {
        self.utxos.len()
    }

    /// Returns the number of UTXOs that are owned by supported addresses.
    pub fn address_utxos_len(&self) -> u64 {
        self.address_utxos.len()
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
        mut utxos_delta: UtxosDelta,
        mut stats: BlockIngestionStats,
    ) -> Slicing<(), BlockHash> {
        let ins_start = performance_counter();
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
                self.partial_stable_block = Some(PartialStableBlock {
                    block,
                    next_tx_idx: tx_idx,
                    next_input_idx,
                    next_output_idx,
                    utxos_delta,
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
        Slicing::Done(block.block_hash())
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
            // NOTE: We're using `inc_performance_counter` here to increment the mock performance
            // counter in the unit tests.
            if inc_performance_counter() >= MAX_INSTRUCTIONS_THRESHOLD {
                return Slicing::Paused(input_idx);
            }

            // Remove the input from the UTXOs. The input *must* exist in the UTXO set.
            let outpoint: OutPoint = (&input.previous_output).into();
            match self.utxos.remove(&(&input.previous_output).into()) {
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
                        let address_balance = self.balances.get(&address).unwrap_or_else(|| {
                            panic!("Address {} must exist in the balances map", address);
                        });

                        match address_balance - txout.value {
                            // Remove the address from the map if balance is zero.
                            0 => self.balances.remove(&address),
                            // Update the balance in the map.
                            balance => self.balances.insert(address.clone(), balance).unwrap(),
                        };

                        utxos_delta.remove(address, outpoint, txout, height);
                    }
                }
                None => {
                    panic!("Outpoint {:?} not found.", input.previous_output);
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

        // TODO: we can maybe avoid the outpoint clone below?
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
    StableBTreeMap::init(
        crate::memory::get_address_utxos_memory(),
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

/// A state for maintaining a stable block that is partially ingested into the UTXO set.
/// Used for time slicing.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq)]
pub struct PartialStableBlock {
    pub block: Block,
    pub next_tx_idx: usize,
    pub next_input_idx: usize,
    pub next_output_idx: usize,
    pub stats: BlockIngestionStats,
    utxos_delta: UtxosDelta,
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
            utxos_delta: UtxosDelta::default(),
            stats: BlockIngestionStats::default(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq, Default)]
pub struct UtxosDelta {
    utxos: BTreeMap<OutPoint, (TxOut, Height)>,
    added_outpoints: BTreeMap<Address, Vec<OutPoint>>,
    removed_outpoints: BTreeMap<Address, Vec<OutPoint>>,
}

impl UtxosDelta {
    fn insert(&mut self, address: Address, outpoint: OutPoint, tx_out: TxOut, height: Height) {
        self.added_outpoints
            .entry(address)
            .or_insert(vec![])
            .push(outpoint.clone());

        let res = self.utxos.insert(outpoint, (tx_out, height));
        assert_eq!(res, None, "Cannot add the same UTXO twice into UtxosDelta");
    }

    fn remove(&mut self, address: Address, outpoint: OutPoint, tx_out: TxOut, height: Height) {
        self.removed_outpoints
            .entry(address)
            .or_insert(vec![])
            .push(outpoint.clone());

        // NOTE: We do not assert here that the UTXO doesn't exist, because here we could be
        // removing a UTXO that was added in the current block, in which case we would have
        // inserted its UTXO earlier into this map.
        self.utxos.insert(outpoint, (tx_out, height));
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
            && is_stable_btreemap_equal(&self.address_utxos, &other.address_utxos)
            && is_stable_btreemap_equal(&self.balances, &other.balances)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{random_p2pkh_address, BlockBuilder, TransactionBuilder};
    use crate::{
        address_utxoset::AddressUtxoSet,
        runtime,
        types::{Network, OutPoint, Txid},
        unstable_blocks::UnstableBlocks,
    };
    use bitcoin::blockdata::{opcodes::all::OP_RETURN, script::Builder};
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

    // If a block is being ingested, `get_balance` returns results as if the ingesting block
    // doesn't exist. The balances are updated only after the block is fully ingested.
    #[test]
    fn get_balance_doesnt_include_changes_from_ingesting_block() {
        let network = Network::Testnet;
        let mut utxo_set = UtxoSet::new(network);
        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);
        let address_3 = random_p2pkh_address(network);

        // A coinbase transaction giving 1000 to address 1.
        let tx_1 = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();

        // Address 1 gives 400 to address 2.
        let tx_2 = TransactionBuilder::new()
            .with_input(OutPoint {
                txid: tx_1.txid(),
                vout: 0,
            })
            .with_output(&address_2, 400)
            .with_output(&address_1, 600)
            .build();

        // Address 1 gives 400 to address 3.
        let tx_3 = TransactionBuilder::new()
            .with_input(OutPoint {
                txid: tx_2.txid(),
                vout: 1,
            })
            .with_output(&address_3, 400)
            .with_output(&address_1, 200)
            .build();

        let block = BlockBuilder::genesis()
            .with_transaction(tx_1)
            .with_transaction(tx_2)
            .with_transaction(tx_3)
            .build();

        let block_hash = block.block_hash();

        // Set a high instruction cost to trigger slicing.
        runtime::set_performance_counter_step(1_000_000_000);

        // Ingestion in progress. `get_balance` should return zero for all the balances.
        assert_eq!(utxo_set.ingest_block(block), Slicing::Paused(()));
        assert_eq!(utxo_set.get_balance(&address_1), 0);
        assert_eq!(utxo_set.get_balance(&address_2), 0);
        assert_eq!(utxo_set.get_balance(&address_3), 0);

        runtime::performance_counter_reset();

        // Ingestion in progress. `get_balance` should return zero for all the balances.
        assert_eq!(utxo_set.ingest_block_continue(), Slicing::Paused(()));
        assert_eq!(utxo_set.get_balance(&address_1), 0);
        assert_eq!(utxo_set.get_balance(&address_2), 0);
        assert_eq!(utxo_set.get_balance(&address_3), 0);

        runtime::performance_counter_reset();

        // Ingestion is done. `get_balance` should return the updated balances.
        assert_eq!(
            utxo_set.ingest_block_continue(),
            Slicing::Done(Some(block_hash))
        );
        assert_eq!(utxo_set.get_balance(&address_1), 200);
        assert_eq!(utxo_set.get_balance(&address_2), 400);
        assert_eq!(utxo_set.get_balance(&address_3), 400);
    }
}
