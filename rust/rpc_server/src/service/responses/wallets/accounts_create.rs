use rsnano_core::Account;
use rsnano_node::{node::Node, wallets::WalletsExt};
use rsnano_rpc_messages::{AccountsCreateArgs, AccountsRpcMessage, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn accounts_create(
    node: Arc<Node>,
    enable_control: bool,
    args: AccountsCreateArgs,
) -> String {
    if !enable_control {
        return to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap();
    }

    let work = args.work.unwrap_or(false);
    let count = args.wallet_with_count.count as usize;
    let wallet = &args.wallet_with_count.wallet;

    let accounts: Result<Vec<Account>, _> = (0..count)
        .map(|_| node.wallets.deterministic_insert2(wallet, work))
        .map(|result| result.map(|public_key| public_key.into()))
        .collect();

    match accounts {
        Ok(accounts) => to_string_pretty(&AccountsRpcMessage::new(accounts)).unwrap(),
        Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::WalletId;
    use rsnano_node::wallets::WalletsExt;
    use std::{thread::sleep, time::Duration};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn accounts_create() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet = WalletId::random();

        node.wallets.create(wallet);

        node.tokio.block_on(async {
            rpc_client
                .accounts_create(wallet, 8, Some(true))
                .await
                .unwrap()
        });

        assert_eq!(
            node.wallets.get_accounts_of_wallet(&wallet).unwrap().len(),
            8
        );

        server.abort();
    }

    #[test]
    fn accounts_create_work_true() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::random();

        node.wallets.create(wallet_id);

        let result = node.tokio.block_on(async {
            rpc_client
                .accounts_create(wallet_id, 1, Some(true))
                .await
                .unwrap()
        });

        assert!(node.wallets.exists(&result.accounts[0].into()));

        sleep(Duration::from_millis(10000));

        assert_ne!(
            node.wallets
                .work_get2(&wallet_id, &result.accounts[0].into())
                .unwrap(),
            0
        );

        server.abort();
    }

    #[test]
    fn accounts_create_work_false() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::random();

        node.wallets.create(wallet_id);

        let result = node.tokio.block_on(async {
            rpc_client
                .accounts_create(wallet_id, 1, Some(false))
                .await
                .unwrap()
        });

        assert!(node.wallets.exists(&result.accounts[0].into()));

        sleep(Duration::from_millis(10000));

        assert_eq!(
            node.wallets
                .work_get2(&wallet_id, &result.accounts[0].into())
                .unwrap(),
            0
        );

        server.abort();
    }

    #[test]
    fn accounts_create_fails_wallet_locked() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::random();

        node.wallets.create(wallet_id);

        node.wallets.lock(&wallet_id).unwrap();

        let result = node
            .tokio
            .block_on(async { rpc_client.accounts_create(wallet_id, 1, None).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet is locked\"".to_string())
        );

        server.abort();
    }

    #[test]
    fn accounts_create_fails_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::random();

        let result = node
            .tokio
            .block_on(async { rpc_client.accounts_create(wallet_id, 1, None).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }

    #[test]
    fn accounts_create_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let wallet = WalletId::random();

        node.wallets.create(wallet);

        let result = node
            .tokio
            .block_on(async { rpc_client.accounts_create(wallet, 8, None).await });

        assert!(result.is_err());

        server.abort();
    }
}
