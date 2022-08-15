use crate::{types::Network, unstable_blocks::UnstableBlocks, utxos::Utxos};
use bitcoin::Block;
use ic_btc_types::Height;
use serde::{Deserialize, Serialize};
use stable_structures::{DefaultMemoryImpl, RestrictedMemory, StableBTreeMap};

/// A structure used to maintain the entire state.
#[derive(Serialize, Deserialize, PartialEq)]
pub struct State {
    // The height of the latest block marked as stable.
    pub height: Height,

    // The UTXOs of all stable blocks since genesis.
    pub utxos: UtxoSet,

    // Blocks inserted, but are not considered stable yet.
    pub unstable_blocks: UnstableBlocks,
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
            height: 0,
            utxos: UtxoSet::new(network),
            unstable_blocks: UnstableBlocks::new(stability_threshold, genesis_block),
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
}

impl PartialEq for UtxoSet {
    fn eq(&self, other: &Self) -> bool {
        // TODO: add the rest
        self.network == other.network
    }
}

impl UtxoSet {
    pub fn new(network: Network) -> Self {
        Self {
            utxos: Utxos::default(),
            address_to_outpoints: init_address_outpoints(),
            network,
        }
    }
}

fn init_address_outpoints() -> StableBTreeMap<RestrictedMemory<DefaultMemoryImpl>, Vec<u8>, Vec<u8>>
{
    StableBTreeMap::init(
        RestrictedMemory::new(DefaultMemoryImpl::default(), 2000..2999),
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
            }

            let mut bytes = vec![];
            ciborium::ser::into_writer(&state, &mut bytes).unwrap();
            let new_state: State = ciborium::de::from_reader(&bytes[..]).unwrap();

            // Verify parts of the state are the same after serialization/deserialization.
            assert_eq!(state.height, new_state.height);
            assert_eq!(state.unstable_blocks, new_state.unstable_blocks);
            assert_eq!(state.utxos.network, new_state.utxos.network);
            assert_eq!(state.utxos.utxos.large_utxos, new_state.utxos.utxos.large_utxos);

            // TODO(EXC-1188): Verify that stable btreemaps are also equal.
        }
    }
}
