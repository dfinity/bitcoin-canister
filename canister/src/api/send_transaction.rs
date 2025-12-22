use crate::{
    charge_cycles, runtime, types::SendTransactionInternalRequest, verify_api_access,
    verify_network, with_state, with_state_mut,
};
use bitcoin::{consensus::Decodable, Transaction};
use ic_btc_interface::{SendTransactionError, SendTransactionRequest};

pub async fn send_transaction(request: SendTransactionRequest) -> Result<(), SendTransactionError> {
    verify_api_access();
    verify_network(request.network.into());

    charge_cycles(with_state(|s| {
        s.fees.send_transaction_base
            + s.fees.send_transaction_per_byte * request.transaction.len() as u128
    }));

    // Decode the transaction as a sanity check that it's valid.
    let tx = Transaction::consensus_decode(&mut request.transaction.as_slice())
        .map_err(|_| SendTransactionError::MalformedTransaction)?;

    runtime::print(&format!("[send_transaction] Tx ID: {}", tx.compute_txid()));

    // Bump the counter for the number of (valid) requests received.
    with_state_mut(|s| {
        s.metrics.send_transaction_count += 1;
    });

    // Use the internal endpoint to send the transaction to the bitcoin network.
    runtime::call_send_transaction_internal(
        with_state(|s| s.blocks_source),
        SendTransactionInternalRequest {
            network: request.network.into(),
            transaction: request.transaction,
        },
    )
    .await
    .expect("Sending transaction bitcoin network must succeed");
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::CanisterArg;
    use bitcoin::{absolute::LockTime, transaction::Version};
    use ic_btc_interface::{Fees, Flag, InitConfig, Network, NetworkInRequest};

    fn empty_transaction() -> Vec<u8> {
        let mut buf = vec![];

        use bitcoin::consensus::Encodable;
        Transaction {
            version: Version(0),
            lock_time: LockTime::from_consensus(0),
            input: vec![],
            output: vec![],
        }
        .consensus_encode(&mut buf)
        .unwrap();

        buf
    }

    #[async_std::test]
    async fn charges_cycles() {
        crate::init(Some(CanisterArg::Init(InitConfig {
            fees: Some(Fees {
                send_transaction_base: 13,
                send_transaction_per_byte: 27,
                ..Default::default()
            }),
            network: Some(Network::Mainnet),
            ..Default::default()
        })));

        let transaction = empty_transaction();
        let transaction_len = transaction.len();

        // The count metric is zero.
        assert_eq!(with_state(|s| s.metrics.send_transaction_count), 0);

        send_transaction(SendTransactionRequest {
            network: NetworkInRequest::Mainnet,
            transaction,
        })
        .await
        .unwrap();

        assert_eq!(
            crate::runtime::get_cycles_balance(),
            13 + 27 * transaction_len as u128
        );

        // The metrics has been updated.
        assert_eq!(with_state(|s| s.metrics.send_transaction_count), 1);
    }

    #[async_std::test]
    async fn invalid_tx_error() {
        crate::init(Some(CanisterArg::Init(InitConfig {
            fees: Some(Fees {
                send_transaction_base: 13,
                send_transaction_per_byte: 27,
                ..Default::default()
            }),
            network: Some(Network::Mainnet),
            ..Default::default()
        })));

        let result = send_transaction(SendTransactionRequest {
            network: NetworkInRequest::Mainnet,
            transaction: vec![1, 2, 3], // Invalid transaction
        })
        .await;
        assert!(result == Err(SendTransactionError::MalformedTransaction));
    }

    #[async_std::test]
    #[should_panic(expected = "Bitcoin API is disabled")]
    async fn send_transaction_access_disabled() {
        crate::init(Some(CanisterArg::Init(InitConfig {
            fees: Some(Fees {
                send_transaction_base: 13,
                send_transaction_per_byte: 27,
                ..Default::default()
            }),
            network: Some(Network::Mainnet),
            api_access: Some(Flag::Disabled),
            ..Default::default()
        })));

        send_transaction(SendTransactionRequest {
            network: NetworkInRequest::Mainnet,
            transaction: vec![1, 2, 3], // Invalid transaction
        })
        .await
        .unwrap();
    }
}
