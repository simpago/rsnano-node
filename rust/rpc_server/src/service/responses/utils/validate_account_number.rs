use rsnano_rpc_messages::SuccessDto;
use serde_json::to_string_pretty;

pub async fn validate_account_number() -> String {
    to_string_pretty(&SuccessDto::new()).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::Account;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn validate_account_number() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        node.tokio.block_on(async {
            rpc_client
                .validate_account_number(Account::zero())
                .await
                .unwrap()
        });

        server.abort();
    }
}
