use rsnano_core::WalletId;
use rsnano_node::wallets::WalletsExt;
use std::time::Duration;
use test_helpers::{assert_timely, setup_rpc_client_and_server, System};

#[test]
fn account_create_options_none() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let result = node.runtime.block_on(async {
        rpc_client
            .account_create(wallet_id, None, None)
            .await
            .unwrap()
    });

    assert!(node.wallets.exists(&result.value.into()));

    server.abort();
}

#[test]
fn account_create_index_max() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let result = node.runtime.block_on(async {
        rpc_client
            .account_create(wallet_id, Some(u32::MAX), None)
            .await
            .unwrap()
    });

    assert!(node.wallets.exists(&result.value.into()));

    server.abort();
}

#[test]
fn account_create_work_true() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let result = node.runtime.block_on(async {
        rpc_client
            .account_create(wallet_id, None, Some(true))
            .await
            .unwrap()
    });

    assert!(node.wallets.exists(&result.value.into()));

    assert_timely(Duration::from_secs(10), || {
        node.wallets
            .work_get2(&wallet_id, &result.value.into())
            .unwrap()
            != 0
    });

    server.abort();
}

#[test]
fn account_create_work_false() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let result = node.runtime.block_on(async {
        rpc_client
            .account_create(wallet_id, None, Some(false))
            .await
            .unwrap()
    });

    assert!(node.wallets.exists(&result.value.into()));

    assert_timely(Duration::from_secs(5), || {
        node.wallets
            .work_get2(&wallet_id, &result.value.into())
            .unwrap()
            == 0
    });

    server.abort();
}

#[test]
fn account_create_fails_without_enable_control() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let result = node
        .runtime
        .block_on(async { rpc_client.account_create(wallet_id, None, None).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"RPC control is disabled\"".to_string())
    );

    server.abort();
}

#[test]
fn account_create_fails_wallet_locked() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    node.wallets.lock(&wallet_id).unwrap();

    let result = node
        .runtime
        .block_on(async { rpc_client.account_create(wallet_id, None, None).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet is locked\"".to_string())
    );

    server.abort();
}

#[test]
fn account_create_fails_wallet_not_found() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    let result = node
        .runtime
        .block_on(async { rpc_client.account_create(wallet_id, None, None).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet not found\"".to_string())
    );

    server.abort();
}
