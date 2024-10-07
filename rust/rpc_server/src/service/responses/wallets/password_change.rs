use rsnano_core::WalletId;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{ErrorDto, SuccessDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn password_change(
    node: Arc<Node>,
    enable_control: bool,
    wallet: WalletId,
    password: String,
) -> String {
    if enable_control {
        match node.wallets.rekey(&wallet, password) {
            Ok(_) => to_string_pretty(&SuccessDto::new()).unwrap(),
            Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::WalletId;
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn password_change() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id: WalletId = 1.into();

        node.wallets.create(wallet_id);

        node.tokio.block_on(async {
            rpc_client
                .password_change(wallet_id, "password".to_string())
                .await
                .unwrap()
        });

        assert!(node.wallets.attempt_password(&wallet_id, "").is_err());
        assert!(node
            .wallets
            .attempt_password(&wallet_id, "password")
            .is_ok());

        server.abort();
    }

    #[test]
    fn password_change_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let wallet_id: WalletId = 1.into();

        node.wallets.create(wallet_id);

        let result = node.tokio.block_on(async {
            rpc_client
                .password_change(wallet_id, "password".to_string())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }

    #[test]
    fn password_change_fails_with_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .password_change(WalletId::zero(), "password".to_string())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }
}
