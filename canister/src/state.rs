use crate::{
    types::{GetSuccessorsResponse, Network},
    unstable_blocks::UnstableBlocks,
    utxos::Utxos,
};
use bitcoin::Block;
use ic_btc_types::Height;
use ic_cdk::export::Principal;
use serde::{Deserialize, Serialize};
use stable_structures::{DefaultMemoryImpl, RestrictedMemory, StableBTreeMap};

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
    // Queues used to communicate with the adapter.
    //   pub adapter_queues: AdapterQueues,

    // Cache for the current fee percentiles.
    //pub fee_percentiles_cache: Option<FeePercentilesCache>,
}

impl State {
    /// Create a new blockchain.
    ///
    /// The `stability_threshold` parameter specifies how many confirmations a
    /// block needs before it is considered stable. Stable blocks are assumed
    /// to be final and are never removed.
    pub fn new(stability_threshold: u32, network: Network, genesis_block: Block) -> Self {
        Self {
            utxos: UtxoSet::new(network),
            unstable_blocks: UnstableBlocks::new(stability_threshold, genesis_block),
            syncing_state: SyncingState::default(),
            blocks_source: Principal::management_canister(),
            //        adapter_queues: AdapterQueues::default(),
            //       fee_percentiles_cache: None,
        }
    }
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
    pub address_to_outpoints: StableBTreeMap<RestrictedMemory<DefaultMemoryImpl>, Vec<u8>, Vec<u8>>,

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
            && is_stable_btreemap_equal(&self.address_to_outpoints, &other.address_to_outpoints)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncingState {
    /// A flag used to ensure that only one request for fetching blocks is
    /// being sent at a time.
    pub is_fetching_blocks: bool,

    /// A response that needs to be processed.
    pub response_to_process: Option<GetSuccessorsResponse>,
}

/// A state for maintaining a stable block that is partially ingested into the UTXO set.
/// Used for time slicing.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq)]
pub struct PartialStableBlock {
    #[serde(serialize_with = "crate::serde::serialize_block")]
    #[serde(deserialize_with = "crate::serde::deserialize_block")]
    pub block: Block,
    pub next_tx_idx: usize,
    pub next_input_idx: usize,
    pub next_output_idx: usize,
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
        }
    }
}

fn init_address_outpoints() -> StableBTreeMap<RestrictedMemory<DefaultMemoryImpl>, Vec<u8>, Vec<u8>>
{
    StableBTreeMap::init(
        crate::memory::get_address_outpoints_memory(),
        MAX_ADDRESS_OUTPOINT_SIZE,
        0, // No values are stored in the map.
    )
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
            stability_threshold in 1..100u32,
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
                crate::store::insert_block(&mut state, block.clone()).unwrap();
                crate::store::ingest_stable_blocks_into_utxoset(&mut state);
            }

            let mut bytes = vec![];
            ciborium::ser::into_writer(&state, &mut bytes).unwrap();
            let new_state: State = ciborium::de::from_reader(&bytes[..]).unwrap();

            // Verify the new state is the same as the old state.
            assert!(state == new_state);
        }
    }
}
