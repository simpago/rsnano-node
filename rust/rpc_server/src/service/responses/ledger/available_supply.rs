use rsnano_core::Account;
use rsnano_node::node::Node;
use rsnano_rpc_messages::AmountDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn available_supply(node: Arc<Node>) -> String {
    let tx = node.store.env.tx_begin_read();
    let genesis_balance =
        node.balance(&node.network_params.ledger.genesis.account_field().unwrap());

    let landing_balance = node.balance(
        &Account::decode_hex("059F68AAB29DE0D3A27443625C7EA9CDDB6517A8B76FE37727EF6A4D76832AD5")
            .unwrap(),
    );

    let faucet_balance = node.balance(
        &Account::decode_hex("8E319CE6F3025E5B2DF66DA7AB1467FE48F1679C13DD43BFDB29FA2E9FC40D3B")
            .unwrap(),
    );

    let burned_balance = node.ledger.account_receivable(
        &tx,
        &Account::decode_account(
            "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
        )
        .unwrap(),
        false,
    );

    let available = genesis_balance - landing_balance - faucet_balance - burned_balance;

    let available_supply = AmountDto::new("available".to_string(), available);

    to_string_pretty(&available_supply).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::Amount;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn available_supply() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.available_supply().await.unwrap() });

        assert_eq!(result.value, Amount::MAX);

        server.abort();
    }
}
