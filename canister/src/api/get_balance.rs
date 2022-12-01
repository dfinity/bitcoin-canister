use crate::{
    charge_cycles,
    runtime::{performance_counter, print},
    types::{Address, GetBalanceRequest},
    unstable_blocks, verify_has_enough_cycles, with_state, with_state_mut,
};
use ic_btc_types::{GetBalanceError, Satoshi};
use std::str::FromStr;

// Various profiling stats for tracking the performance of `get_balance`.
#[derive(Debug, Default)]
struct Stats {
    // The total number of instructions used to process the request.
    ins_total: u64,

    // The number of instructions used to apply the unstable blocks.
    ins_apply_unstable_blocks: u64,
}

/// Retrieves the balance of the given Bitcoin address.
pub fn get_balance(request: GetBalanceRequest) -> Satoshi {
    verify_has_enough_cycles(with_state(|s| s.fees.get_balance_maximum));
    charge_cycles(with_state(|s| s.fees.get_balance));

    get_balance_internal(request).expect("get_balance failed")
}

fn get_balance_internal(request: GetBalanceRequest) -> Result<Satoshi, GetBalanceError> {
    let min_confirmations = request.min_confirmations.unwrap_or(0);
    let address =
        Address::from_str(&request.address).map_err(|_| GetBalanceError::MalformedAddress)?;

    // NOTE: It is safe to sum up the balances here without the risk of overflow.
    // The maximum number of bitcoins is 2.1 * 10^7, which is 2.1* 10^15 satoshis.
    // That is well below the max value of a `u64`.
    let (balance, stats) = with_state(|state| {
        // Retrieve the balance that's pre-computed for stable blocks.
        let mut balance = state.utxos.get_balance(&address);

        let main_chain = unstable_blocks::get_main_chain(&state.unstable_blocks);
        if main_chain.len() < min_confirmations as usize {
            return Err(GetBalanceError::MinConfirmationsTooLarge {
                given: min_confirmations,
                max: main_chain.len() as u32,
            });
        }

        // Apply all the unstable blocks.
        let ins_start = performance_counter();
        let chain_height = state.utxos.next_height() + (main_chain.len() as u32) - 1;
        for (i, block) in main_chain.into_chain().iter().enumerate() {
            let block_height = state.utxos.next_height() + (i as u32);
            let confirmations = chain_height - block_height + 1;

            if confirmations < min_confirmations {
                // The block has fewer confirmations than requested.
                // We can stop now since all remaining blocks will have fewer confirmations.
                break;
            }

            for outpoint in state
                .unstable_blocks
                .get_added_outpoints(&block.block_hash(), &address)
            {
                let (txout, _) = state.unstable_blocks.get_tx_out(outpoint).unwrap();
                balance += txout.value;
            }

            for outpoint in state
                .unstable_blocks
                .get_removed_outpoints(&block.block_hash(), &address)
            {
                let (txout, _) = state.unstable_blocks.get_tx_out(outpoint).unwrap();
                balance -= txout.value;
            }
        }

        let stats = Stats {
            ins_apply_unstable_blocks: performance_counter() - ins_start,
            ins_total: performance_counter(),
        };

        Ok((balance, stats))
    })?;

    // Observe metrics
    with_state_mut(|s| {
        s.metrics.get_balance_total.observe(stats.ins_total);
        s.metrics
            .get_balance_apply_unstable_blocks
            .observe(stats.ins_apply_unstable_blocks);
    });

    // Print the number of instructions it took to process this request.
    print(&format!("[INSTRUCTION COUNT] {:?}: {:?}", request, stats));

    Ok(balance)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        genesis_block, state,
        test_utils::{random_p2pkh_address, BlockBuilder, TransactionBuilder},
        types::{Config, Fees, Network, OutPoint},
        with_state_mut,
    };

    #[test]
    #[should_panic(expected = "get_balance failed: MalformedAddress")]
    fn panics_on_malformed_address() {
        crate::init(Config {
            stability_threshold: 1,
            network: Network::Mainnet,
            ..Default::default()
        });

        get_balance(GetBalanceRequest {
            address: String::from("not an address"),
            min_confirmations: None,
        });
    }

    #[test]
    fn retrieves_the_balance_of_address() {
        let network = Network::Regtest;
        crate::init(Config {
            stability_threshold: 2,
            network,
            ..Default::default()
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
        crate::init(Config {
            stability_threshold: 2,
            network,
            ..Default::default()
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

        crate::init(Config {
            stability_threshold: 2,
            network,
            ..Default::default()
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

    #[test]
    fn charges_cycles() {
        crate::init(Config {
            fees: Fees {
                get_balance: 10,
                ..Default::default()
            },
            ..Default::default()
        });

        get_balance(GetBalanceRequest {
            address: random_p2pkh_address(Network::Regtest).to_string(),
            min_confirmations: None,
        });

        assert_eq!(crate::runtime::get_cycles_balance(), 10);
    }
}
