use rsnano_core::{Account, WalletId};
use rsnano_node::node::Node;
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_contains(node: Arc<Node>, wallet: WalletId, account: Account) -> String {
    let wallet_accounts = match node.wallets.get_accounts_of_wallet(&wallet) {
        Ok(accounts) => accounts,
        Err(e) => return to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    };

    if wallet_accounts.contains(&account) {
        to_string_pretty(&BoolDto::new("exists".to_string(), true)).unwrap()
    } else {
        to_string_pretty(&BoolDto::new("exists".to_string(), false)).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Account, WalletId};
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn wallet_contains_true() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet: WalletId = 1.into();

        node.wallets.create(1.into());

        let account = node
            .wallets
            .deterministic_insert2(&wallet, false)
            .unwrap()
            .into();

        assert!(node.wallets.exists(&account));

        let result = node.tokio.block_on(async {
            rpc_client
                .wallet_contains(wallet, account.into())
                .await
                .unwrap()
        });

        assert_eq!(result.value, true);

        server.abort();
    }

    #[test]
    fn wallet_contains_false() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet: WalletId = 1.into();

        node.wallets.create(1.into());

        let result = node.tokio.block_on(async {
            rpc_client
                .wallet_contains(wallet, Account::zero())
                .await
                .unwrap()
        });

        assert_eq!(result.value, false);

        server.abort();
    }

    #[test]
    fn wallet_contains_fails_with_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .wallet_contains(WalletId::zero(), Account::zero())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }
}
