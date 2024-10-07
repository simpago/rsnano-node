use rsnano_core::Amount;
use rsnano_rpc_messages::AmountDto;
use serde_json::to_string_pretty;

pub async fn nano_to_raw(nano: Amount) -> String {
    to_string_pretty(&AmountDto::new(
        "raw".to_string(),
        Amount::raw(nano.number()),
    ))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::Amount;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn nano_to_raw() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.nano_to_raw(Amount::nano(1)).await.unwrap() });

        assert_eq!(result.value, Amount::raw(1000000000000000000000000000000));

        server.abort();
    }
}
