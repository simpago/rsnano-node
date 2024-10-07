use rsnano_core::{Amount, MXRB_RATIO};
use rsnano_rpc_messages::AmountDto;
use serde_json::to_string_pretty;

pub async fn raw_to_nano(amount: Amount) -> String {
    to_string_pretty(&AmountDto::new(
        "raw".to_string(),
        Amount::nano(amount.number() / *MXRB_RATIO),
    ))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::Amount;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn raw_to_nano() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .raw_to_nano(Amount::raw(1000000000000000000000000000000))
                .await
                .unwrap()
        });

        assert_eq!(result.value, Amount::nano(1));

        server.abort();
    }
}
