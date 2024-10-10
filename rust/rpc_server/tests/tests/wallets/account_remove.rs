use rsnano_core::{Account, WalletId};
use rsnano_node::wallets::WalletsExt;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn account_remove() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet = WalletId::random();

    node.wallets.create(wallet);

    let account = node.wallets.deterministic_insert2(&wallet, false).unwrap();

    assert!(node.wallets.exists(&account));

    node.runtime.block_on(async {
        rpc_client
            .account_remove(wallet, account.into())
            .await
            .unwrap()
    });

    assert!(!node.wallets.exists(&account));

    server.abort();
}

#[test]
fn account_remove_fails_without_enable_control() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let wallet = WalletId::random();

    node.wallets.create(wallet);

    let account = node.wallets.deterministic_insert2(&wallet, false).unwrap();

    assert!(node.wallets.exists(&account));

    let result = node
        .runtime
        .block_on(async { rpc_client.account_remove(wallet, account.into()).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"RPC control is disabled\"".to_string())
    );

    server.abort();
}

#[test]
fn account_remove_fails_wallet_locked() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    node.wallets.lock(&wallet_id).unwrap();

    let result = node
        .runtime
        .block_on(async { rpc_client.account_remove(wallet_id, Account::zero()).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet is locked\"".to_string())
    );

    server.abort();
}

#[test]
fn account_remove_fails_wallet_not_found() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let result = node.runtime.block_on(async {
        rpc_client
            .account_remove(WalletId::zero(), Account::zero())
            .await
    });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet not found\"".to_string())
    );

    server.abort();
}
