use crate::{
    runtime::{performance_counter, print},
    types::GetBalanceRequest,
    with_state,
};
use ic_btc_types::Satoshi;

// Various profiling stats for tracking the performance of `get_balance`.
#[derive(Debug, Default)]
struct Stats {
    // The total number of instructions used to process the request.
    ins_total: u64,

    // The number of instructions used to apply the unstable blocks.
    // NOTE: clippy thinks this is dead code as it's only used in a `print`.
    #[allow(dead_code)]
    ins_apply_unstable_blocks: u64,

    // The number of instructions used to apply the unstable blocks.
    // NOTE: clippy thinks this is dead code as it's only used in a `print`.
    #[allow(dead_code)]
    ins_build_utxos_vec: u64,

    // The number of instructions used to sum all the balances.
    ins_sum_balances: u64,
}

/// Retrieves the balance of the given Bitcoin address.
pub fn get_balance(request: GetBalanceRequest) -> Satoshi {
    let min_confirmations = request.min_confirmations.unwrap_or(0);
    let (get_utxos_res, get_utxos_stats) = with_state(|state| {
        crate::api::get_utxos::get_utxos_internal(
            state,
            &request.address,
            min_confirmations,
            None,
            None,
        )
        .expect("get_balance failed")
    });

    let mut stats = Stats {
        ins_apply_unstable_blocks: get_utxos_stats.ins_apply_unstable_blocks,
        ins_build_utxos_vec: get_utxos_stats.ins_build_utxos_vec,
        ..Default::default()
    };

    // NOTE: It is safe to sum up the balances here without the risk of overflow.
    // The maximum number of bitcoins is 2.1 * 10^7, which is 2.1* 10^15 satoshis.
    // That is well below the max value of a `u64`.
    let ins_start = performance_counter();
    let mut balance = 0;
    for utxo in get_utxos_res.utxos {
        balance += utxo.value;
    }
    stats.ins_sum_balances = performance_counter() - ins_start;
    stats.ins_total = performance_counter();

    // Print the number of instructions it took to process this request.
    print(&format!("[INSTRUCTION COUNT] {:?}: {:?}", request, stats));

    balance
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        genesis_block, state,
        test_utils::{random_p2pkh_address, BlockBuilder, TransactionBuilder},
        types::{InitPayload, Network, OutPoint},
        with_state_mut,
    };

    #[test]
    #[should_panic(expected = "get_balance failed: MalformedAddress")]
    fn panics_on_malformed_address() {
        crate::init(InitPayload {
            stability_threshold: 1,
            network: Network::Mainnet,
            blocks_source: None,
        });

        get_balance(GetBalanceRequest {
            address: String::from("not an address"),
            min_confirmations: None,
        });
    }

    #[test]
    fn retrieves_the_balance_of_address() {
        let network = Network::Regtest;
        crate::init(InitPayload {
            stability_threshold: 2,
            network,
            blocks_source: None,
        });

        // Create a block where 1000 satoshis are given to an address.
        let address = random_p2pkh_address(network);
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address, 1000)
            .build();
        let block = BlockBuilder::with_prev_header(genesis_block(network).header())
            .with_transaction(coinbase_tx)
            .build();

        // Set the state.
        with_state_mut(|state| {
            state::insert_block(state, block).unwrap();
        });

        // With up to one confirmation, expect the address to have a balance 1000.
        for min_confirmations in [None, Some(0), Some(1)].iter() {
            assert_eq!(
                get_balance(GetBalanceRequest {
                    address: address.to_string(),
                    min_confirmations: *min_confirmations
                }),
                1000
            );
        }

        // At two confirmations, the address should have a balance of zero.
        assert_eq!(
            get_balance(GetBalanceRequest {
                address: address.to_string(),
                min_confirmations: Some(2)
            }),
            0
        );
    }

    #[test]
    #[should_panic(expected = "get_balance failed: MinConfirmationsTooLarge { given: 2, max: 1 }")]
    fn panics_on_very_large_confirmations() {
        let network = Network::Regtest;
        crate::init(InitPayload {
            stability_threshold: 2,
            network,
            blocks_source: None,
        });

        let address = random_p2pkh_address(network);

        for min_confirmations in [Some(0), None, Some(1)] {
            assert_eq!(
                get_balance(GetBalanceRequest {
                    address: address.to_string(),
                    min_confirmations
                }),
                0
            );
        }

        // The chain only contains the genesis block, so a min_confirmations of 2
        // should panic, as there aren't that many blocks in the chain.
        get_balance(GetBalanceRequest {
            address: address.to_string(),
            min_confirmations: Some(2),
        });
    }

    #[test]
    fn retrieves_balances_of_addresses_with_different_confirmations() {
        let network = Network::Regtest;

        crate::init(InitPayload {
            stability_threshold: 2,
            network,
            blocks_source: None,
        });

        // Generate addresses.
        let address_1 = random_p2pkh_address(network);
        let address_2 = random_p2pkh_address(network);

        // Create a chain where 1000 satoshis are given to the address_1, then
        // address_1 gives 1000 satoshis to address_2.
        let coinbase_tx = TransactionBuilder::coinbase()
            .with_output(&address_1, 1000)
            .build();
        let block_0 = genesis_block(network);
        let block_1 = BlockBuilder::with_prev_header(block_0.header())
            .with_transaction(coinbase_tx.clone())
            .build();
        let tx = TransactionBuilder::new()
            .with_input(OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_2, 1000)
            .build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header())
            .with_transaction(tx)
            .build();

        // Set the state.
        //        let mut state = State::new(2, network.0, block_0);
        with_state_mut(|state| {
            state::insert_block(state, block_1).unwrap();
            state::insert_block(state, block_2).unwrap();
        });

        // With up to one confirmation, expect address 2 to have a balance 1000, and
        // address 1 to have a balance of 0.
        for min_confirmations in [None, Some(0), Some(1)].iter() {
            assert_eq!(
                get_balance(GetBalanceRequest {
                    address: address_2.to_string(),
                    min_confirmations: *min_confirmations
                }),
                1000
            );

            assert_eq!(
                get_balance(GetBalanceRequest {
                    address: address_1.to_string(),
                    min_confirmations: *min_confirmations
                }),
                0
            );
        }

        // With two confirmations, expect address 2 to have a balance of 0, and address 1 to
        // have a balance of 1000.
        assert_eq!(
            get_balance(GetBalanceRequest {
                address: address_2.to_string(),
                min_confirmations: Some(2)
            }),
            0
        );
        assert_eq!(
            get_balance(GetBalanceRequest {
                address: address_1.to_string(),
                min_confirmations: Some(2)
            }),
            1000
        );
    }
}
