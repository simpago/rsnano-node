use rsnano_core::{Account, WalletId};
use rsnano_node::wallets::WalletsExt;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn account_list() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let wallet = WalletId::random();

    node.wallets.create(wallet);

    let account: Account = node
        .wallets
        .deterministic_insert2(&wallet, false)
        .unwrap()
        .into();

    let result = node
        .runtime
        .block_on(async { server.client.account_list(wallet).await.unwrap() });

    assert_eq!(vec![account], result.accounts);
}

#[test]
fn account_list_fails_wallet_not_found() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node
        .runtime
        .block_on(async { server.client.account_list(WalletId::zero()).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet not found\"".to_string())
    );
}
