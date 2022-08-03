use crate::{unstable_blocks::UnstableBlocks, utxos::Utxos};
use bitcoin::{hashes::Hash, Block, Network, OutPoint, Script, TxOut, Txid};
use ic_btc_types::Height;
use stable_structures::{DefaultMemoryImpl, RestrictedMemory, StableBTreeMap};
/*use crate::{proto, PageMapMemory};
use ic_protobuf::bitcoin::v1;
use ic_replicated_state::bitcoin_state::{
    AdapterQueues, BitcoinState as ReplicatedBitcoinState, FeePercentilesCache, UnstableBlocks,
    UtxoSet as ReplicatedUtxoSet,
};
use ic_replicated_state::page_map::PersistenceError;
use ic_state_layout::{AccessPolicy, ProtoFileWith, RwPolicy};
use std::{convert::TryFrom, path::Path};
*/
/// A structure used to maintain the entire state.
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

/*
    /// Serializes the state to disk at the given path.
    // TODO(EXC-1093): Guard this function with a rust feature. It's only needed in local scripts.
    pub fn serialize(&self, root: &Path) -> Result<(), PersistenceError> {
        // Create the directory if it doesn't exist.
        RwPolicy::check_dir(root).expect("Couldn't create directory.");

        let proto_file: ProtoFileWith<proto::State, RwPolicy> = root.join("state.pbuf").into();
        proto_file
            .serialize(self.into())
            .expect("Serializing state failed");

        // Persist all the memories to disk.
        self.utxos
            .address_to_outpoints
            .get_memory()
            .persist_and_sync_delta(&root.join("address_outpoints.bin"))?;

        self.utxos
            .utxos
            .small_utxos
            .get_memory()
            .persist_and_sync_delta(&root.join("small_utxos.bin"))?;

        self.utxos
            .utxos
            .medium_utxos
            .get_memory()
            .persist_and_sync_delta(&root.join("medium_utxos.bin"))
    }

    // TODO(EXC-1093): Guard this function with a rust feature. It's only needed in local scripts.
    pub fn load(root: &Path) -> Result<Self, PersistenceError> {
        let small_utxos_memory = PageMapMemory::open(&root.join("small_utxos.bin"))?;
        let medium_utxos_memory = PageMapMemory::open(&root.join("medium_utxos.bin"))?;
        let address_to_outpoints_memory = PageMapMemory::open(&root.join("address_outpoints.bin"))?;

        let state_file: ProtoFileWith<proto::State, RwPolicy> = root.join("state.pbuf").into();
        let proto_state = state_file.deserialize_opt().unwrap().unwrap();

        Ok(Self {
            adapter_queues: AdapterQueues::default(),
            height: proto_state.height,
            utxos: UtxoSet::from_proto(
                proto_state.utxos.unwrap(),
                small_utxos_memory,
                medium_utxos_memory,
                address_to_outpoints_memory,
            ),
            unstable_blocks: UnstableBlocks::try_from(proto_state.unstable_blocks.unwrap())
                .unwrap(),
            fee_percentiles_cache: None,
        })
    }*/
}

/*
impl From<ReplicatedBitcoinState> for State {
    fn from(state: ReplicatedBitcoinState) -> Self {
        let utxos_small = state.utxo_set.utxos_small;
        let utxos_medium = state.utxo_set.utxos_medium;
        let address_outpoints = state.utxo_set.address_outpoints;

        Self {
            adapter_queues: state.adapter_queues,
            height: state.stable_height,
            unstable_blocks: state.unstable_blocks,
            utxos: UtxoSet {
                utxos: Utxos {
                    small_utxos: StableBTreeMap::init(
                        PageMapMemory::new(utxos_small),
                        UTXO_KEY_SIZE,
                        UTXO_VALUE_MAX_SIZE_SMALL,
                    ),
                    medium_utxos: StableBTreeMap::init(
                        PageMapMemory::new(utxos_medium),
                        UTXO_KEY_SIZE,
                        UTXO_VALUE_MAX_SIZE_MEDIUM,
                    ),
                    large_utxos: state.utxo_set.utxos_large,
                },
                network: state.utxo_set.network,
                address_to_outpoints: StableBTreeMap::init(
                    PageMapMemory::new(address_outpoints),
                    MAX_ADDRESS_OUTPOINT_SIZE,
                    0,
                ),
            },
            fee_percentiles_cache: state.fee_percentiles_cache,
        }
    }
}

impl From<State> for ReplicatedBitcoinState {
    fn from(state: State) -> Self {
        Self {
            adapter_queues: state.adapter_queues,
            stable_height: state.height,
            unstable_blocks: state.unstable_blocks,
            utxo_set: ReplicatedUtxoSet {
                utxos_small: state.utxos.utxos.small_utxos.get_memory().into_page_map(),
                utxos_medium: state.utxos.utxos.medium_utxos.get_memory().into_page_map(),
                utxos_large: state.utxos.utxos.large_utxos,
                address_outpoints: state
                    .utxos
                    .address_to_outpoints
                    .get_memory()
                    .into_page_map(),
                network: state.utxos.network,
            },
            fee_percentiles_cache: state.fee_percentiles_cache,
        }
    }
}

impl From<&State> for proto::State {
    fn from(state: &State) -> Self {
        proto::State {
            height: state.height,
            utxos: Some(state.utxos.to_proto()),
            unstable_blocks: Some(v1::UnstableBlocks::from(&state.unstable_blocks)),
        }
    }
}
*/
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

pub struct UtxoSet {
    pub utxos: Utxos,
    pub network: Network,
    // An index for fast retrievals of an address's UTXOs.
    pub address_to_outpoints: StableBTreeMap<RestrictedMemory<DefaultMemoryImpl>, Vec<u8>, Vec<u8>>,
}

impl UtxoSet {
    pub fn new(network: Network) -> Self {
        Self {
            utxos: Utxos::default(),
            address_to_outpoints: StableBTreeMap::new(
                RestrictedMemory::new(DefaultMemoryImpl::default(), 2000..2999),
                MAX_ADDRESS_OUTPOINT_SIZE,
                0, // No values are stored in the map.
            ),
            network,
        }
    }

    /*pub fn to_proto(&self) -> proto::UtxoSet {
        proto::UtxoSet {
            large_utxos: self
                .utxos
                .large_utxos
                .iter()
                .map(|(outpoint, (txout, height))| v1::Utxo {
                    outpoint: Some(v1::OutPoint {
                        txid: outpoint.txid.to_vec(),
                        vout: outpoint.vout,
                    }),
                    txout: Some(v1::TxOut {
                        value: txout.value,
                        script_pubkey: txout.script_pubkey.to_bytes(),
                    }),
                    height: *height,
                })
                .collect(),
            network: match self.network {
                Network::Bitcoin => 0,
                Network::Testnet => 1,
                Network::Signet => 2,
                Network::Regtest => 3,
            },
        }
    }

    pub fn from_proto(
        utxos_proto: proto::UtxoSet,
        small_utxos_memory: PageMapMemory,
        medium_utxos_memory: PageMapMemory,
        address_to_outpoints_memory: PageMapMemory,
    ) -> Self {
        let utxos = Utxos {
            small_utxos: StableBTreeMap::load(small_utxos_memory),
            medium_utxos: StableBTreeMap::load(medium_utxos_memory),
            large_utxos: utxos_proto
                .large_utxos
                .into_iter()
                .map(|utxo| {
                    let outpoint = utxo
                        .outpoint
                        .map(|o| {
                            OutPoint::new(
                                Txid::from_hash(Hash::from_slice(&o.txid).unwrap()),
                                o.vout,
                            )
                        })
                        .unwrap();

                    let tx_out = utxo
                        .txout
                        .map(|t| TxOut {
                            value: t.value,
                            script_pubkey: Script::from(t.script_pubkey),
                        })
                        .unwrap();

                    (outpoint, (tx_out, utxo.height))
                })
                .collect(),
        };

        Self {
            utxos,
            address_to_outpoints: StableBTreeMap::load(address_to_outpoints_memory),
            network: match utxos_proto.network {
                0 => Network::Bitcoin,
                1 => Network::Testnet,
                2 => Network::Signet,
                3 => Network::Regtest,
                _ => panic!("Invalid network ID"),
            },
        }
    }*/
}
