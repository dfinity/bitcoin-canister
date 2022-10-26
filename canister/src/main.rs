use ic_btc_canister::types::{Config, HttpRequest, HttpResponse, InitPayload, UpdateConfigRequest};
use ic_btc_types::{
    GetBalanceRequest, GetCurrentFeePercentilesRequest, GetUtxosRequest, GetUtxosResponse,
    MillisatoshiPerByte, Satoshi,
};
use ic_cdk_macros::{heartbeat, init, post_upgrade, pre_upgrade, query, update};
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

#[update]
pub fn get_balance(request: GetBalanceRequest) -> Satoshi {
    ic_btc_canister::get_balance(request)
}

#[update]
pub fn get_utxos(request: GetUtxosRequest) -> GetUtxosResponse {
    ic_btc_canister::get_utxos(request)
}

#[update]
pub fn get_current_fee_percentiles(
    request: GetCurrentFeePercentilesRequest,
) -> Vec<MillisatoshiPerByte> {
    ic_btc_canister::get_current_fee_percentiles(request)
}

#[query]
pub fn get_config() -> Config {
    ic_btc_canister::get_config()
}

#[update]
pub fn update_config(request: UpdateConfigRequest) {
    ic_btc_canister::update_config(request)
}

fn main() {}

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
    /*
    // A default state to use for tests.
    fn default_state() -> ic_btc_canister::state::State {
        State::new(1, Network::Regtest, genesis_block(BitcoinNetwork::Regtest))
    }

    #[test]
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
}
