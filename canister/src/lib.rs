mod address_utxoset;
mod api;
mod block_header_store;
mod blocktree;
mod heartbeat;
mod memory;
mod multi_iter;
mod runtime;
pub mod state;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;
pub mod types;
mod unstable_blocks;
mod utxo_set;
use utxo_set::UtxoSet;

use crate::{
    state::State,
    types::{Block, Config, InitPayload, Network, SetConfigRequest},
};
pub use heartbeat::heartbeat;
use ic_btc_types::{
    GetBalanceRequest, GetCurrentFeePercentilesRequest, GetUtxosRequest, GetUtxosResponse,
    MillisatoshiPerByte, Satoshi,
};
use ic_cdk::export::Principal;
use ic_stable_structures::Memory;
use std::cell::RefCell;
use std::convert::TryInto;
use std::str::FromStr;

thread_local! {
    static STATE: RefCell<Option<State>> = RefCell::new(None);
}

/// A helper method to read the state.
///
/// Precondition: the state is already initialized.
pub fn with_state<R>(f: impl FnOnce(&State) -> R) -> R {
    STATE.with(|cell| f(cell.borrow().as_ref().expect("state not initialized")))
}

// A helper method to mutate the state.
//
// Precondition: the state is already initialized.
fn with_state_mut<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|cell| f(cell.borrow_mut().as_mut().expect("state not initialized")))
}

// A helper method to set the state.
//
// Precondition: the state is _not_ initialized.
fn set_state(state: State) {
    STATE.with(|cell| {
        // Only assert that the state isn't initialized in production.
        // In tests, it is convenient to be able to reset the state.
        #[cfg(target_arch = "wasm32")]
        assert!(
            cell.borrow().is_none(),
            "cannot initialize an already initialized state"
        );
        *cell.borrow_mut() = Some(state)
    });
}

/// Initializes the state of the Bitcoin canister.
pub fn init(payload: InitPayload) {
    set_state(State::new(
        payload
            .stability_threshold
            .try_into()
            .expect("stability threshold too large"),
        payload.network,
        genesis_block(payload.network),
    ));

    if let Some(blocks_source) = payload.blocks_source {
        with_state_mut(|s| s.blocks_source = blocks_source)
    }
}

pub fn get_current_fee_percentiles(
    request: GetCurrentFeePercentilesRequest,
) -> Vec<MillisatoshiPerByte> {
    verify_network(request.network.into());
    api::get_current_fee_percentiles()
}

pub fn get_balance(request: GetBalanceRequest) -> Satoshi {
    verify_network(request.network.into());
    api::get_balance(request.into())
}

pub fn get_utxos(request: GetUtxosRequest) -> GetUtxosResponse {
    verify_network(request.network.into());
    api::get_utxos(request.into())
}

pub fn get_config() -> Config {
    with_state(|s| Config {
        syncing: s.syncing_state.syncing,
    })
}

pub fn set_config(request: SetConfigRequest) {
    // TODO(EXC-1279): Instead of hard-coding a principal, check that the caller is a canister controller.
    if ic_cdk::api::caller()
        != Principal::from_str("5kqj4-ymytp-ozksm-u62pb-po22y-zqqzf-2o4th-5shdt-m5j6r-kgyfi-2qe")
            .unwrap()
    {
        panic!("Unauthorized sender");
    }

    with_state_mut(|s| {
        if let Some(syncing) = request.syncing {
            s.syncing_state.syncing = syncing;
        }
    });
}

pub fn pre_upgrade() {
    // Serialize the state.
    let mut state_bytes = vec![];
    with_state(|state| ciborium::ser::into_writer(state, &mut state_bytes))
        .expect("failed to encode state");

    // Write the length of the serialized bytes to memory, followed by the
    // by the bytes themselves.
    let len = state_bytes.len() as u32;
    let memory = memory::get_upgrades_memory();
    crate::memory::write(&memory, 0, &len.to_le_bytes());
    crate::memory::write(&memory, 4, &state_bytes);
}

pub fn post_upgrade() {
    let memory = memory::get_upgrades_memory();

    // Read the length of the state bytes.
    let mut state_len_bytes = [0; 4];
    memory.read(0, &mut state_len_bytes);
    let state_len = u32::from_le_bytes(state_len_bytes) as usize;

    // Read the bytes
    let mut state_bytes = vec![0; state_len];
    memory.read(4, &mut state_bytes);

    // Deserialize and set the state.
    let state = ciborium::de::from_reader(&*state_bytes).expect("failed to decode state");
    set_state(state);
}

/// Returns the genesis block of the given network.
pub fn genesis_block(network: Network) -> Block {
    Block::new(bitcoin::blockdata::constants::genesis_block(network.into()))
}

// Verifies that the network is equal to the one maintained by this canister's state.
fn verify_network(network: Network) {
    with_state(|state| {
        if state.network() != network {
            panic!("Network must be {}. Found {}", state.network(), network);
        }
    });
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        test_utils::build_regtest_chain,
        types::{Flag, Network},
    };
    use ic_btc_types::NetworkInRequest;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn init_sets_state(
            stability_threshold in 1..200u128,
            network in prop_oneof![
                Just(Network::Mainnet),
                Just(Network::Testnet),
                Just(Network::Regtest),
            ],
        ) {
            init(InitPayload {
                stability_threshold,
                network,
                blocks_source: None
            });

            with_state(|state| {
                assert!(
                    *state == State::new(stability_threshold as u32, network, genesis_block(network))
                );
            });
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1))]
        #[test]
        fn upgrade(
            stability_threshold in 1..100u128,
            num_blocks in 1..250u32,
            num_transactions_in_block in 1..100u32,
        ) {
            let network = Network::Regtest;

            init(InitPayload {
                stability_threshold,
                network,
                blocks_source: None
            });

            let blocks = build_regtest_chain(num_blocks, num_transactions_in_block);

            // Insert all the blocks. Note that we skip the genesis block, as that
            // is already included as part of initializing the state.
            for block in blocks[1..].iter() {
                with_state_mut(|s| {
                    crate::state::insert_block(s, block.clone()).unwrap();
                    crate::state::ingest_stable_blocks_into_utxoset(s);
                });
            }

            // Run the preupgrade hook.
            pre_upgrade();

            // Take out the old state (which also clears the `STATE` singleton).
            let old_state = STATE.with(|cell| cell.take().unwrap());

            // Run the postupgrade hook.
            post_upgrade();

            // The new and old states should be equivalent.
            with_state(|new_state| assert!(new_state == &old_state));
        }
    }

    #[test]
    #[should_panic(expected = "Network must be mainnet. Found testnet")]
    fn get_balance_correct_network() {
        init(InitPayload {
            stability_threshold: 0,
            network: Network::Mainnet,
            blocks_source: None,
        });
        get_balance(GetBalanceRequest {
            address: String::from(""),
            network: NetworkInRequest::Testnet,
            min_confirmations: None,
        });
    }

    #[test]
    #[should_panic(expected = "Network must be mainnet. Found testnet")]
    fn get_utxos_correct_network() {
        init(InitPayload {
            stability_threshold: 0,
            network: Network::Mainnet,
            blocks_source: None,
        });
        get_utxos(GetUtxosRequest {
            address: String::from(""),
            network: NetworkInRequest::Testnet,
            filter: None,
        });
    }

    #[test]
    #[should_panic(expected = "Network must be mainnet. Found testnet")]
    fn get_current_fee_percentiles_correct_network() {
        init(InitPayload {
            stability_threshold: 0,
            network: Network::Mainnet,
            blocks_source: None,
        });
        get_current_fee_percentiles(GetCurrentFeePercentilesRequest {
            network: NetworkInRequest::Testnet,
        });
    }
}
