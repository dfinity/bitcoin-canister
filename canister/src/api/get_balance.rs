use crate::{
    runtime::{performance_counter, print},
    store,
    types::GetBalanceRequest,
    with_state,
};
use ic_btc_types::Satoshi;

/// Retrieves the balance of the given Bitcoin address.
pub fn get_balance(request: GetBalanceRequest) -> Satoshi {
    let res = with_state(|state| {
        let min_confirmations = request.min_confirmations.unwrap_or(0);
        store::get_balance(state, &request.address, min_confirmations).expect("get_balance failed")
    });

    // Print the number of instructions it took to process this request.
    print(&format!(
        "[INSTRUCTION COUNT] {:?}: {}",
        request,
        performance_counter()
    ));
    res
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        test_utils::{random_p2pkh_address, BlockBuilder},
        types::{InitPayload, Network},
        with_state_mut,
    };
    use ic_btc_test_utils::TransactionBuilder;

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
            store::insert_block(state, block).unwrap();
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
        let block_0 = genesis_block(network.into());
        let block_1 = BlockBuilder::with_prev_header(&block_0.header)
            .with_transaction(coinbase_tx.clone())
            .build();
        let tx = TransactionBuilder::new()
            .with_input(bitcoin::OutPoint::new(coinbase_tx.txid(), 0))
            .with_output(&address_2, 1000)
            .build();
        let block_2 = BlockBuilder::with_prev_header(block_1.header())
            .with_transaction(tx)
            .build();

        // Set the state.
        //        let mut state = State::new(2, network.0, block_0);
        with_state_mut(|state| {
            store::insert_block(state, block_1).unwrap();
            store::insert_block(state, block_2).unwrap();
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
