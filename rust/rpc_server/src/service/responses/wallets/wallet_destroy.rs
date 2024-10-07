use rsnano_core::WalletId;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_destroy(node: Arc<Node>, enable_control: bool, wallet: WalletId) -> String {
    if enable_control {
        node.wallets.destroy(&wallet);
        to_string_pretty(&BoolDto::new("destroyed".to_string(), true)).unwrap()
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
    fn wallet_destroy() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id: WalletId = 1.into();

        node.wallets.create(wallet_id);

        assert!(node.wallets.mutex.lock().unwrap().get(&wallet_id).is_some());

        node.tokio
            .block_on(async { rpc_client.wallet_destroy(wallet_id).await.unwrap() });

        assert!(node.wallets.mutex.lock().unwrap().get(&wallet_id).is_none());

        server.abort();
    }

    #[test]
    fn wallet_destroy_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let wallet_id: WalletId = 1.into();

        node.wallets.create(wallet_id);

        assert!(node.wallets.mutex.lock().unwrap().get(&wallet_id).is_some());

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_destroy(wallet_id).await });

        assert!(result.is_err());

        server.abort();
    }
}
