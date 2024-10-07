use rsnano_core::PublicKey;
use rsnano_rpc_messages::AccountRpcMessage;
use serde_json::to_string_pretty;

pub async fn account_get(key: PublicKey) -> String {
    to_string_pretty(&AccountRpcMessage::new("account".to_string(), key.into())).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::{PublicKey, WalletId};
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn account_get() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::random();

        node.wallets.create(wallet_id);

        let result = node
            .tokio
            .block_on(async { rpc_client.account_get(PublicKey::zero()).await.unwrap() });

        assert_eq!(result.value, PublicKey::zero().into());

        server.abort();
    }
}
