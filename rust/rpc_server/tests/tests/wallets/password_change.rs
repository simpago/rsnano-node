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

    node.runtime.block_on(async {
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

    let result = node.runtime.block_on(async {
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

    let result = node.runtime.block_on(async {
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
