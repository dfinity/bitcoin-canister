mod address_utxoset;
mod api;
mod block_header_store;
mod blocktree;
mod guard;
mod heartbeat;
pub mod memory;
mod metrics;
mod multi_iter;
pub mod runtime;
pub mod state;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;
pub mod types;
pub mod unstable_blocks;
mod utxo_set;
mod validation;

use crate::{
    api::set_config::set_config_no_verification,
    runtime::{msg_cycles_accept, msg_cycles_available, print},
    state::State,
    types::{into_bitcoin_network, HttpRequest, HttpResponse},
};
pub use api::get_metrics;
pub use api::send_transaction;
pub use api::set_config;
use candid::{CandidType, Deserialize};
pub use heartbeat::heartbeat;
use ic_btc_interface::{
    Config, Flag, GetBalanceError, GetBalanceRequest, GetBlockHeadersError, GetBlockHeadersRequest,
    GetBlockHeadersResponse, GetCurrentFeePercentilesRequest, GetUtxosError, GetUtxosRequest,
    GetUtxosResponse, InitConfig, MillisatoshiPerByte, Network, Satoshi, SetConfigRequest,
};
use ic_btc_types::Block;
use ic_stable_structures::Memory;
pub use memory::get_memory;
use serde_bytes::ByteBuf;
use state::main_chain_height;
use std::convert::TryInto;
use std::{cell::RefCell, cmp::max};
use utxo_set::UtxoSet;

/// The maximum number of blocks the canister can be behind the tip to be considered synced.
const SYNCED_THRESHOLD: u32 = 2;

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// A helper method to read the state.
///
/// Precondition: the state is already initialized.
pub fn with_state<R>(f: impl FnOnce(&State) -> R) -> R {
    STATE.with(|cell| f(cell.borrow().as_ref().expect("state not initialized")))
}

/// A helper method to mutate the state.
///
/// Precondition: the state is already initialized.
pub fn with_state_mut<R>(f: impl FnOnce(&mut State) -> R) -> R {
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

/// Resets the fetch mutex and discards any in-progress response.
fn reset_syncing_state(state: &mut State) {
    print("Resetting syncing state...");
    state.syncing_state.is_fetching_blocks = false;
    state.syncing_state.response_to_process = None;
}

#[derive(CandidType, Deserialize)]
pub enum CanisterArg {
    #[serde(rename = "init")]
    Init(InitConfig),
    #[serde(rename = "upgrade")]
    Upgrade(Option<SetConfigRequest>),
}

/// Initializes the state of the Bitcoin canister.
pub fn init(init_config: InitConfig) {
    print("Running init...");

    let config = Config::from(init_config);
    set_state(State::new(
        config
            .stability_threshold
            .try_into()
            .expect("stability threshold too large"),
        config.network,
        genesis_block(config.network),
    ));

    with_state_mut(|s| s.blocks_source = config.blocks_source);
    with_state_mut(|s| s.api_access = config.api_access);
    with_state_mut(|s| s.syncing_state.syncing = config.syncing);
    with_state_mut(|s| s.disable_api_if_not_fully_synced = config.disable_api_if_not_fully_synced);
    with_state_mut(|s| s.watchdog_canister = config.watchdog_canister);
    with_state_mut(|s| s.burn_cycles = config.burn_cycles);
    with_state_mut(|s| s.lazily_evaluate_fee_percentiles = config.lazily_evaluate_fee_percentiles);
    with_state_mut(|s| s.fees = config.fees);

    print("...init completed!");
}

pub fn get_current_fee_percentiles(
    request: GetCurrentFeePercentilesRequest,
) -> Vec<MillisatoshiPerByte> {
    verify_api_access();
    verify_network(request.network.into());
    verify_synced();
    api::get_current_fee_percentiles()
}

pub fn get_balance(request: GetBalanceRequest) -> Result<Satoshi, GetBalanceError> {
    verify_api_access();
    verify_network(request.network.into());
    verify_synced();
    api::get_balance(request.into())
}

pub fn get_balance_query(request: GetBalanceRequest) -> Result<Satoshi, GetBalanceError> {
    verify_api_access();
    verify_network(request.network.into());
    verify_synced();
    api::get_balance_query(request.into())
}

pub fn get_utxos(request: GetUtxosRequest) -> Result<GetUtxosResponse, GetUtxosError> {
    verify_api_access();
    verify_network(request.network.into());
    verify_synced();
    api::get_utxos(request.into())
}

pub fn get_utxos_query(request: GetUtxosRequest) -> Result<GetUtxosResponse, GetUtxosError> {
    verify_api_access();
    verify_network(request.network.into());
    verify_synced();
    api::get_utxos_query(request.into())
}

pub fn get_block_headers(
    request: GetBlockHeadersRequest,
) -> Result<GetBlockHeadersResponse, GetBlockHeadersError> {
    verify_api_access();
    verify_network(request.network.into());
    verify_synced();
    api::get_block_headers(request)
}

pub fn get_config() -> Config {
    with_state(|s| Config {
        stability_threshold: s.unstable_blocks.stability_threshold() as u128,
        syncing: s.syncing_state.syncing,
        blocks_source: s.blocks_source,
        network: s.network(),
        fees: s.fees.clone(),
        api_access: s.api_access,
        disable_api_if_not_fully_synced: s.disable_api_if_not_fully_synced,
        watchdog_canister: s.watchdog_canister,
        burn_cycles: s.burn_cycles,
        lazily_evaluate_fee_percentiles: s.lazily_evaluate_fee_percentiles,
    })
}

pub fn get_blockchain_info() -> types::BlockchainInfo {
    with_state(state::blockchain_info)
}

pub fn pre_upgrade() {
    print("Running pre_upgrade...");

    // Serialize the state.
    let mut state_bytes = vec![];
    with_state_mut(|state| {
        // Reset syncing state to ensure the canister
        // is not locked in a fetching blocks state after the upgrade.
        reset_syncing_state(state);

        ciborium::ser::into_writer(state, &mut state_bytes)
    })
    .expect("failed to encode state");

    // Write the length of the serialized bytes to memory, followed by the
    // by the bytes themselves.
    let len = state_bytes.len() as u32;
    let memory = memory::get_upgrades_memory();
    crate::memory::write(&memory, 0, &len.to_le_bytes());
    crate::memory::write(&memory, 4, &state_bytes);
}

pub fn post_upgrade(config_update: Option<SetConfigRequest>) {
    print("Running post_upgrade...");

    let memory = memory::get_upgrades_memory();

    // Read the length of the state bytes.
    let mut state_len_bytes = [0; 4];
    memory.read(0, &mut state_len_bytes);
    let state_len = u32::from_le_bytes(state_len_bytes) as usize;

    // Read the bytes
    let mut state_bytes = vec![0; state_len];
    memory.read(4, &mut state_bytes);

    // Deserialize and set the state.
    let state: State = ciborium::de::from_reader(&*state_bytes).expect("failed to decode state");

    set_state(state);

    // Reset syncing state to ensure the next upgrade works reliably,
    // even if the upgrade event interrupted the canister fetching state.
    with_state_mut(|state| {
        reset_syncing_state(state);
    });

    // Update the state based on the provided configuration.
    if let Some(config_update) = config_update {
        set_config_no_verification(config_update);
    }
}

pub fn http_request(req: HttpRequest) -> HttpResponse {
    let parts: Vec<&str> = req.url.split('?').collect();
    match parts[0] {
        "/metrics" => crate::api::get_metrics(),
        _ => HttpResponse {
            status_code: 404,
            headers: vec![],
            body: ByteBuf::from(String::from("Not found.")),
        },
    }
}

/// Returns the genesis block of the given network.
pub(crate) fn genesis_block(network: Network) -> Block {
    Block::new(bitcoin::blockdata::constants::genesis_block(
        into_bitcoin_network(network),
    ))
}

pub(crate) fn charge_cycles(amount: u128) {
    verify_has_enough_cycles(amount);
    assert_eq!(
        msg_cycles_accept(amount),
        amount,
        "Accepting cycles must succeed"
    );
}

/// Panics if the request contains less than the amount of cycles given.
pub(crate) fn verify_has_enough_cycles(amount: u128) {
    if msg_cycles_available() < amount {
        panic!(
            "Received {} cycles. {} cycles are required.",
            msg_cycles_available(),
            amount
        );
    }
}

// Verifies that the network is equal to the one maintained by this canister's state.
fn verify_network(network: Network) {
    with_state(|state| {
        if state.network() != network {
            panic!("Network must be {}. Found {}", state.network(), network);
        }
    });
}

// Verifies that the access to bitcoin apis is enabled.
fn verify_api_access() {
    with_state(|state| {
        if state.api_access == Flag::Disabled {
            panic!("Bitcoin API is disabled");
        }
    });
}

// Verifies if the difference between the maximum height
// of all block headers and the maximum height of all unstable
// blocks is at most the SYNCED_THRESHOLD.
fn verify_synced() {
    with_state(|state| {
        if state.disable_api_if_not_fully_synced == Flag::Disabled {
            return;
        }

        if !is_synced() {
            panic!("Canister state is not fully synced.");
        }
    });
}

/// Returns true if the canister is synced with the network, false otherwise.
pub(crate) fn is_synced() -> bool {
    with_state(|state| {
        let main_chain_height = main_chain_height(state);
        main_chain_height + SYNCED_THRESHOLD
            >= max(
                state
                    .unstable_blocks
                    .next_block_headers_max_height()
                    .unwrap_or(0),
                main_chain_height,
            )
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use ic_btc_interface::{Fees, Network, NetworkInRequest};
    use ic_btc_test_utils::build_regtest_chain;
    use proptest::prelude::*;
    use state::ResponseToProcess;

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
            init(InitConfig {
                stability_threshold: Some(stability_threshold),
                network: Some(network),
                ..Default::default()
            });

            with_state(|state| {
                assert!(
                    *state == State::new(stability_threshold as u32, network, genesis_block(network))
                );
            });
        }
    }

    #[test_strategy::proptest(ProptestConfig::with_cases(1))]
    fn upgrade(
        #[strategy(1..100u128)] stability_threshold: u128,
        #[strategy(1..250u32)] num_blocks: u32,
        #[strategy(1..100u32)] num_transactions_in_block: u32,
    ) {
        let network = Network::Regtest;

        init(InitConfig {
            stability_threshold: Some(stability_threshold),
            network: Some(network),
            ..Default::default()
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
        post_upgrade(None);

        // The new and old states should be equivalent.
        with_state(|new_state| assert!(new_state == &old_state));
    }

    #[test_strategy::proptest(ProptestConfig::with_cases(1))]
    fn upgrade_with_config(#[strategy(1..100u128)] stability_threshold: u128) {
        let network = Network::Regtest;

        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(network),
            ..Default::default()
        });

        // Run the preupgrade hook.
        pre_upgrade();

        // Run the postupgrade hook, setting the new stability threshold.
        post_upgrade(Some(SetConfigRequest {
            stability_threshold: Some(stability_threshold),
            ..Default::default()
        }));

        // The config has been updated with the new stability threshold.
        assert_eq!(get_config().stability_threshold, stability_threshold);
    }

    #[test]
    fn test_upgrade_resets_sync_state() {
        let network = Network::Regtest;
        init(InitConfig {
            stability_threshold: Some(144),
            network: Some(network),
            ..Default::default()
        });

        // Simulate a state where the canister is fetching blocks.
        with_state_mut(|state| {
            state.syncing_state.is_fetching_blocks = true;
            state.syncing_state.response_to_process =
                Some(ResponseToProcess::Complete(Default::default())); // Some fake response.
        });

        // Upgrade the canister.
        pre_upgrade();
        post_upgrade(None);

        // The syncing state should be reset.
        with_state(|s| {
            assert!(!s.syncing_state.is_fetching_blocks); // No longer fetching blocks.
            assert!(s.syncing_state.response_to_process.is_none()); // No response to process.
        });
    }

    #[test]
    #[should_panic(expected = "Network must be mainnet. Found testnet")]
    fn get_balance_incorrect_network() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            ..Default::default()
        });
        get_balance(GetBalanceRequest {
            address: String::from(""),
            network: NetworkInRequest::Testnet,
            min_confirmations: None,
        })
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Network must be mainnet. Found testnet")]
    fn get_balance_query_incorrect_network() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            ..Default::default()
        });
        get_balance_query(GetBalanceRequest {
            address: String::from(""),
            network: NetworkInRequest::Testnet,
            min_confirmations: None,
        })
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Network must be mainnet. Found testnet")]
    fn get_utxos_incorrect_network() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            ..Default::default()
        });
        get_utxos(GetUtxosRequest {
            address: String::from(""),
            network: NetworkInRequest::Testnet,
            filter: None,
        })
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Network must be mainnet. Found testnet")]
    fn get_utxos_query_incorrect_network() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            ..Default::default()
        });
        get_utxos_query(GetUtxosRequest {
            address: String::from(""),
            network: NetworkInRequest::Testnet,
            filter: None,
        })
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Network must be mainnet. Found testnet")]
    fn get_current_fee_percentiles_incorrect_network() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            ..Default::default()
        });
        get_current_fee_percentiles(GetCurrentFeePercentilesRequest {
            network: NetworkInRequest::Testnet,
        });
    }

    #[test]
    #[should_panic(expected = "Network must be mainnet. Found testnet")]
    fn get_block_headers_incorrect_network() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            ..Default::default()
        });
        get_block_headers(GetBlockHeadersRequest {
            start_height: 0,
            end_height: None,
            network: NetworkInRequest::Testnet,
        })
        .unwrap();
    }

    #[test]
    fn test_verify_has_enough_cycles_does_not_panic_with_enough_cycles() {
        verify_has_enough_cycles(1_000);
    }

    #[test]
    #[should_panic(
        expected = "Received 170141183460469231731687303715884105727 cycles. 340282366920938463463374607431768211455 cycles are required."
    )]
    fn test_verify_has_enough_cycles_panics_with_not_enough_cycles() {
        verify_has_enough_cycles(u128::MAX);
    }

    #[test]
    #[should_panic(expected = "Bitcoin API is disabled")]
    fn get_balance_access_disabled() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            api_access: Some(Flag::Disabled),
            ..Default::default()
        });
        get_balance(GetBalanceRequest {
            address: String::from(""),
            network: NetworkInRequest::Mainnet,
            min_confirmations: None,
        })
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Bitcoin API is disabled")]
    fn get_balance_query_access_disabled() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            api_access: Some(Flag::Disabled),
            ..Default::default()
        });
        get_balance_query(GetBalanceRequest {
            address: String::from(""),
            network: NetworkInRequest::Mainnet,
            min_confirmations: None,
        })
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Bitcoin API is disabled")]
    fn get_utxos_access_disabled() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            api_access: Some(Flag::Disabled),
            ..Default::default()
        });
        get_utxos(GetUtxosRequest {
            address: String::from(""),
            network: NetworkInRequest::Mainnet,
            filter: None,
        })
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Bitcoin API is disabled")]
    fn get_block_headers_access_disabled() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            api_access: Some(Flag::Disabled),
            ..Default::default()
        });
        get_block_headers(GetBlockHeadersRequest {
            start_height: 3,
            end_height: None,
            network: NetworkInRequest::Mainnet,
        })
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Bitcoin API is disabled")]
    fn get_utxos_query_access_disabled() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            api_access: Some(Flag::Disabled),
            ..Default::default()
        });
        get_utxos_query(GetUtxosRequest {
            address: String::from(""),
            network: NetworkInRequest::Mainnet,
            filter: None,
        })
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Bitcoin API is disabled")]
    fn get_current_fee_percentiles_access_disabled() {
        init(InitConfig {
            stability_threshold: Some(0),
            network: Some(Network::Mainnet),
            api_access: Some(Flag::Disabled),
            ..Default::default()
        });
        get_current_fee_percentiles(GetCurrentFeePercentilesRequest {
            network: NetworkInRequest::Mainnet,
        });
    }

    #[test]
    fn init_sets_syncing_flag() {
        init(InitConfig {
            syncing: Some(Flag::Disabled),
            ..Default::default()
        });

        with_state(|s| {
            assert_eq!(s.syncing_state.syncing, Flag::Disabled);
        });

        init(InitConfig {
            syncing: Some(Flag::Enabled),
            ..Default::default()
        });

        with_state(|s| {
            assert_eq!(s.syncing_state.syncing, Flag::Enabled);
        });
    }

    #[test]
    fn init_sets_disable_api_if_not_fully_synced() {
        init(InitConfig {
            disable_api_if_not_fully_synced: Some(Flag::Disabled),
            ..Default::default()
        });

        with_state(|s| {
            assert_eq!(s.disable_api_if_not_fully_synced, Flag::Disabled);
        });

        init(InitConfig {
            disable_api_if_not_fully_synced: Some(Flag::Enabled),
            ..Default::default()
        });

        with_state(|s| {
            assert_eq!(s.disable_api_if_not_fully_synced, Flag::Enabled);
        });
    }

    #[test]
    fn get_blockchain_info_returns_correct_info() {
        let network = Network::Mainnet;
        init(InitConfig {
            stability_threshold: Some(1),
            network: Some(network),
            ..Default::default()
        });

        let genesis = genesis_block(network);
        let tip_info = get_blockchain_info();

        // After init, the tip is the Bitcoin genesis block for the configured network.
        assert_eq!(tip_info.height, 0);
        assert_eq!(tip_info.block_hash, genesis.block_hash().to_vec());
        assert_eq!(tip_info.timestamp, genesis.header().time);
        assert_eq!(tip_info.difficulty, genesis.difficulty(network));
        // Genesis block has 1 coinbase output.
        assert_eq!(tip_info.utxos_length, 1);
    }

    #[test]
    fn get_blockchain_info_succeeds_when_api_disabled() {
        init(InitConfig {
            api_access: Some(Flag::Disabled),
            ..Default::default()
        });

        let info = get_blockchain_info();
        assert_eq!(info.height, 0);
        assert_eq!(info.utxos_length, 1);
    }

    #[test]
    fn init_applies_default_fees_when_not_explicitly_provided() {
        let custom = Fees {
            get_utxos_base: 123,
            ..Default::default()
        };
        let test_cases = [
            (Network::Testnet, None, Fees::testnet()),
            (Network::Mainnet, None, Fees::mainnet()),
            (Network::Regtest, None, Fees::default()),
            (Network::Testnet, Some(custom.clone()), custom.clone()),
            (Network::Mainnet, Some(custom.clone()), custom.clone()),
            (Network::Regtest, Some(custom.clone()), custom),
        ];
        for (network, provided_fees, expected_fees) in test_cases {
            init(InitConfig {
                network: Some(network),
                fees: provided_fees.clone(),
                ..Default::default()
            });

            with_state(|s| assert_eq!(s.fees, expected_fees));
        }
    }
}
