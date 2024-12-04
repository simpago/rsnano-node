#[cfg(test)]
mod tests {
    use rsnano_core::{Account, WalletId};
    use rsnano_node::wallets::WalletsExt;
    use std::time::Duration;
    use test_helpers::{assert_timely, setup_rpc_client_and_server, System};

    #[test]
    fn work_set() {
        let mut system = System::new();
        let node = system.make_node();

        let server = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::zero();
        node.wallets.create(wallet_id);

        node.runtime.block_on(async {
            server
                .client
                .work_set(wallet_id, Account::zero(), 1.into())
                .await
                .unwrap()
        });

        assert_timely(Duration::from_secs(5), || {
            node.wallets
                .work_get2(&wallet_id, &Account::zero().into())
                .unwrap()
                != 0
        });
    }

    #[test]
    fn work_set_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let server = setup_rpc_client_and_server(node.clone(), false);

        let result = node.runtime.block_on(async {
            server
                .client
                .work_set(WalletId::zero(), Account::zero(), 1.into())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );
    }

    #[test]
    fn work_set_fails_with_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let server = setup_rpc_client_and_server(node.clone(), true);

        let result = node.runtime.block_on(async {
            server
                .client
                .work_set(WalletId::zero(), Account::zero(), 1.into())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );
    }
}
