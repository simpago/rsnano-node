use rsnano_core::{Account, WalletId};
use rsnano_ledger::DEV_GENESIS_ACCOUNT;
use rsnano_node::wallets::WalletsExt;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn wallet_add_watch() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::zero();

    node.wallets.create(wallet_id);

    node.runtime.block_on(async {
        rpc_client
            .wallet_add_watch(wallet_id, vec![*DEV_GENESIS_ACCOUNT])
            .await
            .unwrap()
    });

    assert!(node.wallets.exists(&(*DEV_GENESIS_ACCOUNT).into()));

    server.abort();
}

#[test]
fn wallet_add_watch_without_enable_control() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let wallet_id = WalletId::zero();

    node.wallets.create(wallet_id);

    let result = node.runtime.block_on(async {
        rpc_client
            .wallet_add_watch(wallet_id, vec![Account::zero()])
            .await
    });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"RPC control is disabled\"".to_string())
    );

    server.abort();
}
