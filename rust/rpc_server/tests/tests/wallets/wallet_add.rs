use rsnano_core::{PublicKey, RawKey, WalletId};
use rsnano_node::wallets::WalletsExt;
use std::time::Duration;
use test_helpers::{assert_timely, setup_rpc_client_and_server, System};

#[test]
fn account_create_index_none() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let private_key = RawKey::random();
    let public_key: PublicKey = (&private_key).try_into().unwrap();

    node.runtime.block_on(async {
        rpc_client
            .wallet_add(wallet_id, private_key, None)
            .await
            .unwrap()
    });

    assert!(node
        .wallets
        .get_accounts_of_wallet(&wallet_id)
        .unwrap()
        .contains(&public_key.into()));

    server.abort();
}

#[test]
fn account_create_fails_without_enable_control() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let private_key = RawKey::random();

    let result = node
        .runtime
        .block_on(async { rpc_client.wallet_add(wallet_id, private_key, None).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"RPC control is disabled\"".to_string())
    );

    server.abort();
}

#[test]
fn wallet_add_fails_with_wallet_not_found() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let result = node.runtime.block_on(async {
        rpc_client
            .wallet_add(WalletId::zero(), RawKey::zero(), None)
            .await
    });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet not found\"".to_string())
    );

    server.abort();
}

#[test]
fn wallet_add_work_true() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let private_key = RawKey::random();

    let result = node.runtime.block_on(async {
        rpc_client
            .wallet_add(wallet_id, private_key, Some(true))
            .await
            .unwrap()
    });

    assert_timely(Duration::from_secs(5), || {
        node.wallets
            .work_get2(&wallet_id, &result.value.into())
            .unwrap()
            != 0
    });

    server.abort();
}

#[test]
fn wallet_add_work_false() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::random();

    node.wallets.create(wallet_id);

    let private_key = RawKey::random();

    let result = node.runtime.block_on(async {
        rpc_client
            .wallet_add(wallet_id, private_key, Some(false))
            .await
            .unwrap()
    });

    assert_timely(Duration::from_secs(5), || {
        node.wallets
            .work_get2(&wallet_id, &result.value.into())
            .unwrap()
            == 0
    });

    server.abort();
}
