use rsnano_core::WalletId;
use rsnano_node::wallets::WalletsExt;
use rsnano_rpc_messages::AccountsCreateArgs;
use std::time::Duration;
use test_helpers::{assert_timely, setup_rpc_client_and_server, System};

#[test]
fn accounts_create() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let wallet = WalletId::random();

    node.wallets.create(wallet);

    node.runtime
        .block_on(async { server.client.accounts_create(wallet, 8).await.unwrap() });

    assert_eq!(
        node.wallets.get_accounts_of_wallet(&wallet).unwrap().len(),
        8
    );
}

#[test]
fn accounts_create_default_with_precomputed_work() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let result = node.runtime.block_on(async {
        server
            .client
            .accounts_create_args(
                AccountsCreateArgs::build(wallet_id, 1)
                    .precompute_work(true)
                    .finish(),
            )
            .await
            .unwrap()
    });

    assert!(node.wallets.exists(&result.accounts[0].into()));

    assert_timely(Duration::from_secs(5), || {
        node.wallets
            .work_get2(&wallet_id, &result.accounts[0].into())
            .unwrap()
            != 0
    });
}

#[test]
fn accounts_create_without_precomputed_work() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let args = AccountsCreateArgs::build(wallet_id, 1)
        .precompute_work(false)
        .finish();

    let result = node
        .runtime
        .block_on(async { server.client.accounts_create_args(args).await.unwrap() });

    assert!(node.wallets.exists(&result.accounts[0].into()));

    assert_timely(Duration::from_secs(5), || {
        node.wallets
            .work_get2(&wallet_id, &result.accounts[0].into())
            .unwrap()
            == 0
    });
}

#[test]
fn accounts_create_fails_wallet_locked() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    node.wallets.lock(&wallet_id).unwrap();

    let result = node
        .runtime
        .block_on(async { server.client.accounts_create(wallet_id, 1).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet is locked\"".to_string())
    );
}

#[test]
fn accounts_create_fails_wallet_not_found() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    let result = node
        .runtime
        .block_on(async { server.client.accounts_create(wallet_id, 1).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet not found\"".to_string())
    );
}

#[test]
fn accounts_create_fails_without_enable_control() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let wallet = WalletId::random();

    node.wallets.create(wallet);

    let result = node
        .runtime
        .block_on(async { server.client.accounts_create(wallet, 8).await });

    assert!(result.is_err());
}
