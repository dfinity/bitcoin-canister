use ic_btc_canister::{
    state::State,
    store,
    types::{HttpRequest, HttpResponse, InitPayload},
};
use ic_btc_types::{GetBalanceError, GetUtxosError, GetUtxosResponse, UtxosFilter};
use ic_cdk_macros::{heartbeat, init, post_upgrade, pre_upgrade, query};
use serde_bytes::ByteBuf;

mod metrics;

#[init]
fn init(payload: InitPayload) {
    ic_btc_canister::init(payload);
}

#[pre_upgrade]
fn pre_upgrade() {
    ic_btc_canister::pre_upgrade();
}

#[post_upgrade]
fn post_upgrade() {
    ic_btc_canister::post_upgrade();
}

#[heartbeat]
async fn heartbeat() {
    ic_btc_canister::heartbeat().await
}

fn main() {}

// The maximum number of UTXOs that are allowed to be included in a single
// `GetUtxosResponse`.
//
// Given the size of a `Utxo` is 48 bytes, this means that the size of a single
// response can be ~500KiB (considering the size of remaining fields and
// potential overhead for the candid serialization). This is still quite below
// the max response payload size of 2MiB that the IC needs to respect.

// The value also conforms to the interface spec which requires that no more
// than 100_000 `Utxo`s are returned in a single response.
const MAX_UTXOS_PER_RESPONSE: usize = 10_000;

/// Retrieves the balance of the given Bitcoin address.
pub fn get_balance(
    state: &State,
    address: &str,
    min_confirmations: Option<u32>,
) -> Result<u64, GetBalanceError> {
    let min_confirmations = min_confirmations.unwrap_or(0);

    store::get_balance(state, address, min_confirmations)
}

pub fn get_utxos(
    state: &State,
    address: &str,
    filter: Option<UtxosFilter>,
) -> Result<GetUtxosResponse, GetUtxosError> {
    match filter {
        None => {
            // No filter is specified. Return all UTXOs for the address.
            store::get_utxos(state, address, 0, None, Some(MAX_UTXOS_PER_RESPONSE))
        }
        Some(UtxosFilter::MinConfirmations(min_confirmations)) => {
            // Return UTXOs with the requested number of confirmations.
            store::get_utxos(
                state,
                address,
                min_confirmations,
                None,
                Some(MAX_UTXOS_PER_RESPONSE),
            )
        }
        Some(UtxosFilter::Page(page)) => store::get_utxos(
            state,
            address,
            0,
            Some(page.to_vec()),
            Some(MAX_UTXOS_PER_RESPONSE),
        ),
    }
}
/*
pub fn send_transaction(
    state: &mut State,
    request: SendTransactionRequest,
) -> Result<(), SendTransactionError> {
    if Transaction::deserialize(&request.transaction).is_err() {
        return Err(SendTransactionError::MalformedTransaction);
    }

    match state
        .adapter_queues
        .push_request(BitcoinAdapterRequestWrapper::SendTransactionRequest(
            InternalSendTransactionRequest {
                transaction: request.transaction,
            },
        )) {
        Ok(()) => {}
        Err(_err @ BitcoinStateError::QueueFull { .. }) => {
            return Err(SendTransactionError::QueueFull);
        }
        // TODO(EXC-1098): Refactor the `push_request` method to not return these
        // errors to avoid this `unreachable` statement.
        Err(BitcoinStateError::FeatureNotEnabled)
        | Err(BitcoinStateError::NonMatchingResponse { .. }) => unreachable!(),
    }

    Ok(())
}*/

#[query]
pub fn http_request(req: HttpRequest) -> HttpResponse {
    let parts: Vec<&str> = req.url.split('?').collect();
    match parts[0] {
        "/metrics" => metrics::handle_metrics_request(),
        _ => HttpResponse {
            status_code: 404,
            headers: vec![],
            body: ByteBuf::from(String::from("Not found.")),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitcoin::{blockdata::constants::genesis_block, Block, Network as BitcoinNetwork};
    use ic_btc_canister::types::Network;
    use ic_btc_test_utils::{
        random_p2pkh_address, random_p2tr_address, BlockBuilder, TransactionBuilder,
    };
    use ic_btc_types::{OutPoint, Utxo};

    // A default state to use for tests.
    fn default_state() -> State {
        State::new(1, Network::Regtest, genesis_block(BitcoinNetwork::Regtest))
    }

    #[test]
    fn get_utxos_from_existing_utxo_set() {
        for network in [
            (Network::Mainnet, BitcoinNetwork::Bitcoin),
            (Network::Regtest, BitcoinNetwork::Regtest),
            (Network::Testnet, BitcoinNetwork::Testnet),
        ]
        .iter()
        {
            // Generate an address.
            let address = random_p2pkh_address(network.1);

            // Create a genesis block where 1000 satoshis are given to the address.
            let coinbase_tx = TransactionBuilder::coinbase()
                .with_output(&address, 1000)
                .build();
            let genesis_block = BlockBuilder::genesis()
                .with_transaction(coinbase_tx.clone())
                .build();

            // Set the state.
            let state = State::new(0, network.0, genesis_block.clone());

            assert_eq!(
                get_utxos(&state, &address.to_string(), None),
                Ok(GetUtxosResponse {
                    utxos: vec![Utxo {
                        outpoint: OutPoint {
                            txid: coinbase_tx.txid().to_vec(),
                            vout: 0
                        },
                        value: 1000,
                        height: 0,
                    }],
                    tip_block_hash: genesis_block.block_hash().to_vec(),
                    tip_height: 0,
                    next_page: None,
                })
            );
        }
    }

    #[test]
    fn get_balance_malformed_address() {
        assert_eq!(
            get_balance(&default_state(), "not an address", None),
            Err(GetBalanceError::MalformedAddress)
        );
    }

    #[test]
    fn get_utxos_malformed_address() {
        assert_eq!(
            get_utxos(&default_state(), "not an address", None),
            Err(GetUtxosError::MalformedAddress)
        );
    }

    #[test]
    fn get_balance_test() {
        for network in [
            (Network::Mainnet, BitcoinNetwork::Bitcoin),
            (Network::Regtest, BitcoinNetwork::Regtest),
            (Network::Testnet, BitcoinNetwork::Testnet),
        ]
        .iter()
        {
            // Generate addresses.
            let address_1 = random_p2pkh_address(network.1);

            let address_2 = random_p2pkh_address(network.1);

            // Create a genesis block where 1000 satoshis are given to the address_1, followed
            // by a block where address_1 gives 1000 satoshis to address_2.
            let coinbase_tx = TransactionBuilder::coinbase()
                .with_output(&address_1, 1000)
                .build();
            let block_0 = BlockBuilder::genesis()
                .with_transaction(coinbase_tx.clone())
                .build();
            let tx = TransactionBuilder::new()
                .with_input(bitcoin::OutPoint::new(coinbase_tx.txid(), 0))
                .with_output(&address_2, 1000)
                .build();
            let block_1 = BlockBuilder::with_prev_header(block_0.header)
                .with_transaction(tx.clone())
                .build();

            // Set the state.
            let mut state = State::new(2, network.0, block_0);
            store::insert_block(&mut state, block_1).unwrap();

            // With up to one confirmation, expect address 2 to have a balance 1000, and
            // address 1 to have a balance of 0.
            for min_confirmations in [None, Some(0), Some(1)].iter() {
                assert_eq!(
                    get_balance(&state, &address_2.to_string(), *min_confirmations),
                    Ok(1000)
                );

                assert_eq!(
                    get_balance(&state, &address_1.to_string(), *min_confirmations),
                    Ok(0)
                );
            }

            // With two confirmations, expect address 2 to have a balance of 0, and address 1 to
            // have a balance of 1000.
            assert_eq!(get_balance(&state, &address_2.to_string(), Some(2)), Ok(0));
            assert_eq!(
                get_balance(&state, &address_1.to_string(), Some(2)),
                Ok(1000)
            );

            // With >= 2 confirmations, we should get an error as that's higher than
            // the chain's height.
            for i in 3..10 {
                assert_eq!(
                    get_balance(&state, &address_2.to_string(), Some(i)),
                    Err(GetBalanceError::MinConfirmationsTooLarge { given: i, max: 2 })
                );
                assert_eq!(
                    get_balance(&state, &address_1.to_string(), Some(i)),
                    Err(GetBalanceError::MinConfirmationsTooLarge { given: i, max: 2 })
                );
            }
        }
    }

    #[test]
    fn get_utxos_min_confirmations() {
        for network in [
            (Network::Mainnet, BitcoinNetwork::Bitcoin),
            (Network::Regtest, BitcoinNetwork::Regtest),
            (Network::Testnet, BitcoinNetwork::Testnet),
        ]
        .iter()
        {
            // Generate addresses.
            let address_1 = random_p2pkh_address(network.1);

            let address_2 = random_p2pkh_address(network.1);

            // Create a genesis block where 1000 satoshis are given to the address_1, followed
            // by a block where address_1 gives 1000 satoshis to address_2.
            let coinbase_tx = TransactionBuilder::coinbase()
                .with_output(&address_1, 1000)
                .build();
            let block_0 = BlockBuilder::genesis()
                .with_transaction(coinbase_tx.clone())
                .build();
            let tx = TransactionBuilder::new()
                .with_input(bitcoin::OutPoint::new(coinbase_tx.txid(), 0))
                .with_output(&address_2, 1000)
                .build();
            let block_1 = BlockBuilder::with_prev_header(block_0.header)
                .with_transaction(tx.clone())
                .build();

            // Set the state.
            let mut state = State::new(2, network.0, block_0.clone());
            store::insert_block(&mut state, block_1.clone()).unwrap();

            // With up to one confirmation, expect address 2 to have one UTXO, and
            // address 1 to have no UTXOs.
            for min_confirmations in [None, Some(0), Some(1)].iter() {
                assert_eq!(
                    get_utxos(
                        &state,
                        &address_2.to_string(),
                        min_confirmations.map(UtxosFilter::MinConfirmations),
                    ),
                    Ok(GetUtxosResponse {
                        utxos: vec![Utxo {
                            outpoint: OutPoint {
                                txid: tx.txid().to_vec(),
                                vout: 0,
                            },
                            value: 1000,
                            height: 1,
                        }],
                        tip_block_hash: block_1.block_hash().to_vec(),
                        tip_height: 1,
                        next_page: None,
                    })
                );

                assert_eq!(
                    get_utxos(
                        &state,
                        &address_1.to_string(),
                        min_confirmations.map(UtxosFilter::MinConfirmations),
                    ),
                    Ok(GetUtxosResponse {
                        utxos: vec![],
                        tip_block_hash: block_1.block_hash().to_vec(),
                        tip_height: 1,
                        next_page: None,
                    })
                );
            }

            // With two confirmations, expect address 2 to have no UTXOs, and address 1 to
            // have one UTXO.
            assert_eq!(
                get_utxos(
                    &state,
                    &address_2.to_string(),
                    Some(UtxosFilter::MinConfirmations(2))
                ),
                Ok(GetUtxosResponse {
                    utxos: vec![],
                    tip_block_hash: block_0.block_hash().to_vec(),
                    tip_height: 0,
                    next_page: None,
                })
            );
            assert_eq!(
                get_utxos(
                    &state,
                    &address_1.to_string(),
                    Some(UtxosFilter::MinConfirmations(2))
                ),
                Ok(GetUtxosResponse {
                    utxos: vec![Utxo {
                        outpoint: OutPoint {
                            txid: coinbase_tx.txid().to_vec(),
                            vout: 0,
                        },
                        value: 1000,
                        height: 0,
                    }],
                    tip_block_hash: block_0.block_hash().to_vec(),
                    tip_height: 0,
                    next_page: None,
                })
            );

            // With >= 2 confirmations, we should get an error as that's higher than
            // the chain's height.
            for i in 3..10 {
                assert_eq!(
                    get_utxos(
                        &state,
                        &address_2.to_string(),
                        Some(UtxosFilter::MinConfirmations(i))
                    ),
                    Err(GetUtxosError::MinConfirmationsTooLarge { given: i, max: 2 })
                );
                assert_eq!(
                    get_utxos(
                        &state,
                        &address_1.to_string(),
                        Some(UtxosFilter::MinConfirmations(i))
                    ),
                    Err(GetUtxosError::MinConfirmationsTooLarge { given: i, max: 2 })
                );
            }
        }
    }

    #[test]
    fn get_utxos_returns_results_in_descending_height_order() {
        for network in [
            (Network::Mainnet, BitcoinNetwork::Bitcoin),
            (Network::Regtest, BitcoinNetwork::Regtest),
            (Network::Testnet, BitcoinNetwork::Testnet),
        ]
        .iter()
        {
            // Generate addresses.
            let address_1 = random_p2tr_address(network.1);

            let address_2 = random_p2pkh_address(network.1);

            // Create a blockchain which alternates between giving some BTC to
            // address_1 and address_2 based on whether we're creating an even
            // or an odd height block.
            let num_blocks = 10;
            let mut prev_block: Option<Block> = None;
            let mut transactions = vec![];
            let mut blocks = vec![];
            for i in 0..num_blocks {
                let tx = if i % 2 == 0 {
                    TransactionBuilder::coinbase()
                        .with_output(&address_1, i + 1)
                        .build()
                } else {
                    TransactionBuilder::coinbase()
                        .with_output(&address_2, i + 1)
                        .build()
                };
                transactions.push(tx.clone());
                let block = match prev_block {
                    Some(b) => BlockBuilder::with_prev_header(b.header)
                        .with_transaction(tx.clone())
                        .build(),
                    None => BlockBuilder::genesis().with_transaction(tx.clone()).build(),
                };
                blocks.push(block.clone());
                prev_block = Some(block);
            }

            // Set the state.
            let mut state = State::new(2, network.0, blocks[0].clone());
            for block in blocks[1..].iter() {
                store::insert_block(&mut state, block.clone()).unwrap();
            }

            // We expect that address_1 has `Utxo`s on all even heights and
            // address_2 on all odd heights, both in descending order.
            let mut expected_utxos_address_1 = vec![];
            let mut expected_utxos_address_2 = vec![];
            for i in (0..num_blocks).rev() {
                let expected_utxo = Utxo {
                    outpoint: OutPoint {
                        txid: transactions[i as usize].txid().to_vec(),
                        vout: 0,
                    },
                    value: i + 1,
                    height: i as u32,
                };
                if i % 2 == 0 {
                    expected_utxos_address_1.push(expected_utxo)
                } else {
                    expected_utxos_address_2.push(expected_utxo);
                }
            }

            assert_eq!(
                get_utxos(&state, &address_1.to_string(), None,),
                Ok(GetUtxosResponse {
                    utxos: expected_utxos_address_1,
                    tip_block_hash: blocks.last().unwrap().block_hash().to_vec(),
                    tip_height: num_blocks as u32 - 1,
                    next_page: None,
                })
            );

            assert_eq!(
                get_utxos(&state, &address_2.to_string(), None,),
                Ok(GetUtxosResponse {
                    utxos: expected_utxos_address_2,
                    tip_block_hash: blocks.last().unwrap().block_hash().to_vec(),
                    tip_height: num_blocks as u32 - 1,
                    next_page: None,
                })
            );
        }
    }

    /*#[test]
    fn send_transaction_malformed_transaction() {
        assert_eq!(
            send_transaction(
                &mut default_state(),
                SendTransactionRequest {
                    transaction: vec![1, 2, 3],
                    network: BtcTypesNetwork::Testnet,
                }
            ),
            Err(SendTransactionError::MalformedTransaction)
        );
    }

    #[test]
    fn send_transaction_adds_request_to_adapter_queue() {
        let mut state = default_state();

        // Create a fake transaction that passes verification check.
        let tx = TransactionBuilder::coinbase()
            .with_output(&random_p2tr_address(Network::Testnet), 1_000)
            .build();

        assert_eq!(state.adapter_queues.num_requests(), 0);

        let _result = send_transaction(
            &mut state,
            SendTransactionRequest {
                transaction: tx.serialize(),
                network: BtcTypesNetwork::Testnet,
            },
        );

        assert_eq!(state.adapter_queues.num_requests(), 1);
    }*/

    #[test]
    fn support_taproot_addresses() {
        for network in [
            (Network::Mainnet, BitcoinNetwork::Bitcoin),
            (Network::Regtest, BitcoinNetwork::Regtest),
            (Network::Testnet, BitcoinNetwork::Testnet),
        ]
        .iter()
        {
            let address = random_p2tr_address(network.1);

            // Create a genesis block where 1000 satoshis are given to a taproot address.
            let coinbase_tx = TransactionBuilder::coinbase()
                .with_output(&address, 1000)
                .build();
            let block_0 = BlockBuilder::genesis()
                .with_transaction(coinbase_tx.clone())
                .build();

            let state = State::new(0, network.0, block_0.clone());

            // Assert that the UTXOs of the taproot address can be retrieved.
            assert_eq!(
                get_utxos(&state, &address.to_string(), None),
                Ok(GetUtxosResponse {
                    utxos: vec![Utxo {
                        outpoint: OutPoint {
                            txid: coinbase_tx.txid().to_vec(),
                            vout: 0,
                        },
                        value: 1000,
                        height: 0,
                    }],
                    tip_block_hash: block_0.block_hash().to_vec(),
                    tip_height: 0,
                    next_page: None,
                })
            );
        }
    }
}
