use rsnano_core::WalletId;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_locked(node: Arc<Node>, wallet: WalletId) -> String {
    match node.wallets.valid_password(&wallet) {
        Ok(valid) => to_string_pretty(&BoolDto::new("locked".to_string(), !valid)).unwrap(),
        Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::WalletId;
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn wallet_locked_false() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id: WalletId = 1.into();

        node.wallets.create(wallet_id);

        assert_eq!(node.wallets.valid_password(&wallet_id).unwrap(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_locked(wallet_id).await.unwrap() });

        assert_eq!(result.value, false);

        server.abort();
    }

    #[test]
    fn wallet_locked_true() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let wallet_id: WalletId = 1.into();

        node.wallets.create(wallet_id);

        node.wallets.lock(&wallet_id).unwrap();

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_locked(wallet_id).await.unwrap() });

        assert_eq!(result.value, true);

        server.abort();
    }

    #[test]
    fn wallet_locked_fails_with_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_locked(WalletId::zero()).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }
}
