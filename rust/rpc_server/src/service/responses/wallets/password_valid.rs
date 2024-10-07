use rsnano_core::WalletId;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn password_valid(node: Arc<Node>, wallet: WalletId) -> String {
    match node.wallets.valid_password(&wallet) {
        Ok(valid) => to_string_pretty(&BoolDto::new("valid".to_string(), valid)).unwrap(),
        Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::WalletId;
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn password_valid() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let wallet_id: WalletId = 1.into();

        node.wallets.create(wallet_id);

        let _ = node.wallets.enter_password(wallet_id, "password");

        let result = node
            .tokio
            .block_on(async { rpc_client.password_valid(wallet_id).await.unwrap() });

        assert_eq!(result.value, false);

        let _ = node.wallets.enter_password(wallet_id, "");

        let result = node
            .tokio
            .block_on(async { rpc_client.password_valid(wallet_id).await.unwrap() });

        assert_eq!(result.value, true);

        server.abort();
    }

    #[test]
    fn password_valid_fails_with_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.password_valid(WalletId::zero()).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }
}
