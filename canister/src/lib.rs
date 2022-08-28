mod address_utxoset;
mod api;
mod blocktree;
mod heartbeat;
mod memory;
pub mod state;
pub mod store;
#[cfg(test)]
mod test_utils;
pub mod types;
mod unstable_blocks;
mod utxos;
mod utxoset;

use crate::{state::State, types::InitPayload};
pub use api::get_balance;
pub use api::get_utxos;
use bitcoin::blockdata::constants::genesis_block;
pub use heartbeat::heartbeat;
use stable_structures::Memory;
use std::cell::RefCell;
use std::convert::TryInto;

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
        genesis_block(payload.network.into()),
    ));

    if let Some(blocks_source) = payload.blocks_source {
        with_state_mut(|s| s.blocks_source = blocks_source)
    }
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
    memory.write(0, &len.to_le_bytes());
    memory.write(4, &state_bytes);
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{test_utils::build_regtest_chain, types::Network};
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
                    *state == State::new(stability_threshold as u32, network, genesis_block(network.into()))
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
                    crate::store::insert_block(s, block.clone()).unwrap();
                    crate::store::write_stable_blocks_into_utxoset(s);
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
}
