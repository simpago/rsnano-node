use rsnano_core::{Amount, WalletId, DEV_GENESIS_KEY};
use rsnano_node::wallets::WalletsExt;
use test_helpers::{send_block, setup_rpc_client_and_server, System};

#[test]
fn wallet_info() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let wallet = WalletId::zero();

    node.wallets.create(wallet);
    node.wallets
        .insert_adhoc2(&wallet, &DEV_GENESIS_KEY.private_key(), false)
        .unwrap();
    node.wallets.deterministic_insert2(&wallet, false).unwrap();

    send_block(node.clone());

    let result = node
        .runtime
        .block_on(async { server.client.wallet_info(wallet).await.unwrap() });

    assert_eq!(result.balance, Amount::MAX - Amount::raw(1));
    assert_eq!(result.pending, Amount::raw(1));
    assert_eq!(result.receivable, Amount::raw(1));
    assert_eq!(result.accounts_block_count, 2.into());
    assert_eq!(result.accounts_cemented_block_count, 1.into());
    assert_eq!(result.adhoc_count, 1.into());
    assert_eq!(result.deterministic_count, 1.into());
    assert_eq!(result.deterministic_index, 1.into());
    assert_eq!(result.accounts_count, 2.into());
}

#[test]
fn wallet_info_fails_with_wallet_not_found() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let result = node
        .runtime
        .block_on(async { server.client.wallet_info(WalletId::zero()).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet not found\"".to_string())
    );
}
