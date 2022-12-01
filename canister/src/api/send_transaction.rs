use crate::{
    charge_cycles, runtime, types::SendTransactionInternalRequest, verify_network, with_state,
    with_state_mut,
};
use bitcoin::{consensus::Decodable, Transaction};
use ic_btc_types::SendTransactionRequest;

pub async fn send_transaction(request: SendTransactionRequest) {
    verify_network(request.network.into());

    charge_cycles(with_state(|s| {
        s.fees.send_transaction_base
            + s.fees.send_transaction_per_byte * request.transaction.len() as u128
    }));

    // Decode the transaction as a sanity check that it's valid.
    let tx = Transaction::consensus_decode(request.transaction.as_slice())
        .expect("Cannot decode transaction");

    runtime::print(&format!("[send_transaction] Tx ID: {}", tx.txid()));

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
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::{Config, Fees, Network};
    use ic_btc_types::NetworkInRequest;

    fn empty_transaction() -> Vec<u8> {
        let mut buf = vec![];

        use bitcoin::consensus::Encodable;
        Transaction {
            version: 0,
            lock_time: 0,
            input: vec![],
            output: vec![],
        }
        .consensus_encode(&mut buf)
        .unwrap();

        buf
    }

    #[async_std::test]
    async fn charges_cycles() {
        crate::init(Config {
            fees: Fees {
                send_transaction_base: 13,
                send_transaction_per_byte: 27,
                ..Default::default()
            },
            network: Network::Mainnet,
            ..Default::default()
        });

        let transaction = empty_transaction();
        let transaction_len = transaction.len();

        // The count metric is zero.
        assert_eq!(with_state(|s| s.metrics.send_transaction_count), 0);

        send_transaction(SendTransactionRequest {
            network: NetworkInRequest::Mainnet,
            transaction,
        })
        .await;

        assert_eq!(
            crate::runtime::get_cycles_balance(),
            13 + 27 * transaction_len as u64
        );

        // The metrics has been updated.
        assert_eq!(with_state(|s| s.metrics.send_transaction_count), 1);
    }

    #[async_std::test]
    #[should_panic(expected = "Cannot decode transaction")]
    async fn invalid_tx_panics() {
        crate::init(Config {
            fees: Fees {
                send_transaction_base: 13,
                send_transaction_per_byte: 27,
                ..Default::default()
            },
            network: Network::Mainnet,
            ..Default::default()
        });

        send_transaction(SendTransactionRequest {
            network: NetworkInRequest::Mainnet,
            transaction: vec![1, 2, 3], // Invalid transaction
        })
        .await;
    }
}
