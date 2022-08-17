mod address_utxoset;
mod blocktree;
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
use bitcoin::blockdata::constants::genesis_block;
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<Option<State>> = RefCell::new(None);
}

/// A helper method to read the state.
///
/// Precondition: the state is already initialized.
pub fn with_state<R>(f: impl FnOnce(&State) -> R) -> R {
    STATE.with(|cell| f(cell.borrow().as_ref().expect("state not initialized")))
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
        payload.stability_threshold,
        payload.network,
        genesis_block(payload.network.into()),
    ))
}

#[cfg(test)]
mod test {
    use proptest::prelude::*;
    use super::*;
    use crate::types::Network;

    proptest! {
        #[test]
        fn init_sets_state(
            stability_threshold in 1..200u32,
            network in prop_oneof![
                Just(Network::Mainnet),
                Just(Network::Testnet),
                Just(Network::Regtest),
            ],
        ) {
            init(InitPayload {
                stability_threshold,
                network,
            });

            with_state(|state| {
                assert!(
                    *state == State::new(stability_threshold, network, genesis_block(network.into()))
                );
            });
        }
    }
}
