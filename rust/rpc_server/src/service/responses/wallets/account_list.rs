use rsnano_core::WalletId;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{AccountsRpcMessage, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_list(node: Arc<Node>, wallet: WalletId) -> String {
    match node.wallets.get_accounts_of_wallet(&wallet) {
        Ok(accounts) => {
            let account_list = AccountsRpcMessage::new(accounts);
            to_string_pretty(&account_list).unwrap()
        }
        Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Account, WalletId};
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn account_list() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let wallet = WalletId::random();

        node.wallets.create(wallet);

        let account: Account = node
            .wallets
            .deterministic_insert2(&wallet, false)
            .unwrap()
            .into();

        let result = node
            .tokio
            .block_on(async { rpc_client.account_list(wallet).await.unwrap() });

        assert_eq!(vec![account], result.accounts);

        server.abort();
    }

    #[test]
    fn account_list_fails_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::random();

        let result = node
            .tokio
            .block_on(async { rpc_client.account_list(WalletId::zero()).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }
}
