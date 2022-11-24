use crate::{
    charge_cycles, runtime, types::SendTransactionInternalRequest, verify_network, with_state,
};
use ic_btc_types::SendTransactionRequest;

pub async fn send_transaction(request: SendTransactionRequest) {
    verify_network(request.network.into());

    charge_cycles(with_state(|s| {
        s.fees.send_transaction_base
            + s.fees.send_transaction_per_byte * request.transaction.len() as u128
    }));

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

        send_transaction(SendTransactionRequest {
            network: NetworkInRequest::Mainnet,
            transaction: vec![1, 2, 3],
        })
        .await;

        assert_eq!(crate::runtime::get_cycles_balance(), 13 + 27 * 3);
    }
}
