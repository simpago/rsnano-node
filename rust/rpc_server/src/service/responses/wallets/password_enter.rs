use rsnano_core::WalletId;
use rsnano_node::{node::Node, wallets::WalletsExt};
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn password_enter(node: Arc<Node>, wallet: WalletId, password: String) -> String {
    match node.wallets.enter_password(wallet, &password) {
        Ok(_) => to_string_pretty(&BoolDto::new("valid".to_string(), true)).unwrap(),
        Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::WalletId;
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn password_enter() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id: WalletId = 1.into();

        node.wallets.create(wallet_id);
        node.wallets.lock(&wallet_id).unwrap();
        assert!(node
            .wallets
            .deterministic_insert2(&wallet_id, false)
            .is_err());

        node.tokio.block_on(async {
            rpc_client
                .password_enter(wallet_id, "".to_string())
                .await
                .unwrap()
        });

        assert!(node
            .wallets
            .deterministic_insert2(&wallet_id, false)
            .is_ok());

        server.abort();
    }

    #[test]
    fn password_enter_fails_with_invalid_password() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let wallet_id: WalletId = 1.into();

        node.wallets.create(wallet_id);

        let result = node.tokio.block_on(async {
            rpc_client
                .password_enter(wallet_id, "password".to_string())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Invalid password\"".to_string())
        );

        server.abort();
    }

    #[test]
    fn password_enter_fails_with_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
            rpc_client
                .password_enter(WalletId::zero(), "password".to_string())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }
}
