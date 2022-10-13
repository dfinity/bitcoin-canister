use crate::{
    address_utxoset::AddressUtxoSet,
    block_header_store::BlockHeaderStore,
    blocktree::BlockDoesNotExtendTree,
    memory::Memory,
    types::{
        Address, Block, BlockHash, GetSuccessorsCompleteResponse, GetSuccessorsPartialResponse,
        Network, Slicing,
    },
    unstable_blocks::{self, UnstableBlocks},
    utxos::Utxos,
    utxoset,
};
use ic_btc_types::{Height, MillisatoshiPerByte};
use ic_cdk::export::Principal;
use ic_stable_structures::StableBTreeMap;
use serde::{Deserialize, Serialize};

/// A structure used to maintain the entire state.
// NOTE: `PartialEq` is only available in tests as it would be impractically
// expensive in production.
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct State {
    /// The UTXOs of all stable blocks since genesis.
    pub utxos: UtxoSet,

    /// Blocks inserted, but are not considered stable yet.
    pub unstable_blocks: UnstableBlocks,

    /// State used for syncing new blocks.
    pub syncing_state: SyncingState,

    /// The canister from which blocks are retrieved.
    /// Defaults to the management canister in production.
    pub blocks_source: Principal,

    /// Cache for the current fee percentiles.
    pub fee_percentiles_cache: Option<FeePercentilesCache>,

    /// A store containing all the stable blocks' headers.
    pub stable_block_headers: BlockHeaderStore,
}

impl State {
    /// Create a new blockchain.
    ///
    /// The `stability_threshold` parameter specifies how many confirmations a
    /// block needs before it is considered stable. Stable blocks are assumed
    /// to be final and are never removed.
    pub fn new(stability_threshold: u32, network: Network, genesis_block: Block) -> Self {
        let utxos = UtxoSet::new(network);
        let unstable_blocks = UnstableBlocks::new(&utxos, stability_threshold, genesis_block);

        Self {
            utxos,
            unstable_blocks,
            syncing_state: SyncingState::default(),
            blocks_source: Principal::management_canister(),
            fee_percentiles_cache: None,
            stable_block_headers: BlockHeaderStore::init(),
        }
    }

    pub fn network(&self) -> Network {
        self.utxos.network
    }

    /// The height of the latest stable block.
    pub fn stable_height(&self) -> Height {
        self.utxos.next_height
    }

    /// Returns the UTXO set of a given bitcoin address.
    pub fn get_utxos(&self, address: Address) -> AddressUtxoSet<'_> {
        AddressUtxoSet::new(address, &self.utxos, &self.unstable_blocks)
    }
}

/// Inserts a block into the state.
/// Returns an error if the block doesn't extend any known block in the state.
pub fn insert_block(state: &mut State, block: Block) -> Result<(), BlockDoesNotExtendTree> {
    unstable_blocks::push(&mut state.unstable_blocks, &state.utxos, block)
}

/// Pops any blocks in `UnstableBlocks` that are considered stable and ingests them to the UTXO set.
///
/// NOTE: This method does a form of time-slicing to stay within the instruction limit, and
/// multiple calls may be required for all the stable blocks to be ingested.
///
/// Returns a bool indicating whether or not the state has changed.
pub fn ingest_stable_blocks_into_utxoset(state: &mut State) -> bool {
    let prev_state = (
        state.utxos.next_height,
        &state.utxos.partial_stable_block.clone(),
    );
    let has_state_changed = |state: &State| -> bool {
        prev_state != (state.utxos.next_height, &state.utxos.partial_stable_block)
    };

    // Finish ingesting the stable block that's partially ingested, if that exists.
    match utxoset::ingest_block_continue(&mut state.utxos) {
        Slicing::Paused(()) => return has_state_changed(state),
        Slicing::Done => {}
    }

    // Check if there are any stable blocks and ingest those into the UTXO set.
    while let Some(new_stable_block) = unstable_blocks::pop(&mut state.unstable_blocks) {
        // Store the block's header.
        state
            .stable_block_headers
            .insert(&new_stable_block, state.utxos.next_height);

        match utxoset::ingest_block(&mut state.utxos, new_stable_block) {
            Slicing::Paused(()) => return has_state_changed(state),
            Slicing::Done => {}
        }
    }

    has_state_changed(state)
}

pub fn main_chain_height(state: &State) -> Height {
    unstable_blocks::get_main_chain(&state.unstable_blocks).len() as u32 + state.utxos.next_height
        - 1
}

pub fn get_unstable_blocks(state: &State) -> Vec<&Block> {
    unstable_blocks::get_blocks(&state.unstable_blocks)
}

// The size of an outpoint in bytes.
const OUTPOINT_TX_ID_SIZE: u32 = 32; // The size of the transaction ID.
const OUTPOINT_VOUT_SIZE: u32 = 4; // The size of a transaction's vout.
const OUTPOINT_SIZE: u32 = OUTPOINT_TX_ID_SIZE + OUTPOINT_VOUT_SIZE;

// The maximum size in bytes of a bitcoin script for it to be considered "small".
const TX_OUT_SCRIPT_MAX_SIZE_SMALL: u32 = 25;

// The maximum size in bytes of a bitcoin script for it to be considered "medium".
const TX_OUT_SCRIPT_MAX_SIZE_MEDIUM: u32 = 201;

// A transaction output's value in satoshis is a `u64`, which is 8 bytes.
const TX_OUT_VALUE_SIZE: u32 = 8;

const TX_OUT_MAX_SIZE_SMALL: u32 = TX_OUT_SCRIPT_MAX_SIZE_SMALL + TX_OUT_VALUE_SIZE;

const TX_OUT_MAX_SIZE_MEDIUM: u32 = TX_OUT_SCRIPT_MAX_SIZE_MEDIUM + TX_OUT_VALUE_SIZE;

// The height is a `u32`, which is 4 bytes.
const HEIGHT_SIZE: u32 = 4;

/// The size of a key in the UTXOs map, which is an outpoint.
pub const UTXO_KEY_SIZE: u32 = OUTPOINT_SIZE;

/// The max size of a value in the "small UTXOs" map.
pub const UTXO_VALUE_MAX_SIZE_SMALL: u32 = TX_OUT_MAX_SIZE_SMALL + HEIGHT_SIZE;

/// The max size of a value in the "medium UTXOs" map.
pub const UTXO_VALUE_MAX_SIZE_MEDIUM: u32 = TX_OUT_MAX_SIZE_MEDIUM + HEIGHT_SIZE;

// The longest addresses are bech32 addresses, and a bech32 string can be at most 90 chars.
// See https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki
const MAX_ADDRESS_SIZE: u32 = 90;
const MAX_ADDRESS_OUTPOINT_SIZE: u32 = MAX_ADDRESS_SIZE + OUTPOINT_SIZE;

#[derive(Serialize, Deserialize)]
pub struct UtxoSet {
    pub utxos: Utxos,

    pub network: Network,

    // An index for fast retrievals of an address's UTXOs.
    // NOTE: Stable structures don't need to be serialized.
    #[serde(skip, default = "init_address_outpoints")]
    pub address_to_outpoints: StableBTreeMap<Memory, Vec<u8>, Vec<u8>>,

    // A map of an address and its current balance.
    // NOTE: Stable structures don't need to be serialized.
    #[serde(skip, default = "init_balances")]
    pub balances: StableBTreeMap<Memory, Address, u64>,

    /// The height of the block that will be ingested next.
    // NOTE: The `next_height` is stored, rather than the current height, because:
    //   * The `UtxoSet` is initialized as empty with no blocks.
    //   * The height of the genesis block is defined as zero.
    //
    // Rather than making this an optional to handle the case where the UTXO set is empty, we
    // instead store the `next_height` to avoid having this special case.
    pub next_height: Height,

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

/// A response awaiting to be processed.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum ResponseToProcess {
    /// A complete response that is ready to be processed.
    Complete(GetSuccessorsCompleteResponse),

    /// A partial response that needs more follow-up requests for it to be complete.
    /// The partial response is stored along with the number of pages of the complete
    /// response that has been processed.
    Partial(GetSuccessorsPartialResponse, u8),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncingState {
    /// A flag used to ensure that only one request for fetching blocks is
    /// being sent at a time.
    pub is_fetching_blocks: bool,

    /// A response that needs to be processed.
    pub response_to_process: Option<ResponseToProcess>,
}

/// Various profiling stats for tracking the performance of block ingestion.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq, Default)]
pub struct BlockIngestionStats {
    /// The number of rounds it took to ingest the block.
    pub num_rounds: u32,

    /// The total number of instructions used to ingest the block.
    pub ins_total: u64,

    /// The number of instructions used to remove the transaction inputs.
    pub ins_remove_inputs: u64,

    /// The number of instructions used to insert the transaction outputs.
    pub ins_insert_outputs: u64,

    /// The number of instructions used to compute the txids.
    pub ins_txids: u64,

    /// The number of instructions used to insert new utxos.
    pub ins_insert_utxos: u64,
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

/// Cache for storing last calculated fee percentiles
///
/// Stores last tip block hash and fee percentiles associated with it.
#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeePercentilesCache {
    pub tip_block_hash: BlockHash,
    pub fee_percentiles: Vec<MillisatoshiPerByte>,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::build_chain;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]
        #[test]
        fn serialize_deserialize_state(
            stability_threshold in 1..150u32,
            network in prop_oneof![
                Just(Network::Mainnet),
                Just(Network::Testnet),
                Just(Network::Regtest),
            ],
            num_blocks in 1..250u32,
            num_transactions_in_block in 1..100u32,
        ) {
            let blocks = build_chain(network, num_blocks, num_transactions_in_block);

            let mut state = State::new(stability_threshold, network, blocks[0].clone());

            for block in blocks[1..].iter() {
                insert_block(&mut state, block.clone()).unwrap();
                ingest_stable_blocks_into_utxoset(&mut state);
            }

            let mut bytes = vec![];
            ciborium::ser::into_writer(&state, &mut bytes).unwrap();
            let new_state: State = ciborium::de::from_reader(&bytes[..]).unwrap();

            // Verify the new state is the same as the old state.
            assert!(state == new_state);
        }
    }
}
