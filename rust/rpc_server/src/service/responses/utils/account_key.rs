use rsnano_core::Account;
use rsnano_rpc_messages::KeyRpcMessage;
use serde_json::to_string_pretty;

pub async fn account_key(account: Account) -> String {
    to_string_pretty(&KeyRpcMessage::new(account.into())).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Account, WalletId};
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn account_key() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::random();

        node.wallets.create(wallet_id);

        let result = node
            .tokio
            .block_on(async { rpc_client.account_key(Account::zero()).await.unwrap() });

        assert_eq!(result.key, Account::zero().into());

        server.abort();
    }
}
