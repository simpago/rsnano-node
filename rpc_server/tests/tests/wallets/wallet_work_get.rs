use rsnano_core::{PublicKey, RawKey, WalletId, WorkNonce};
use rsnano_node::wallets::WalletsExt;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn wallet_work_get() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let wallet = WalletId::zero();
    let private_key = RawKey::zero();
    let public_key = PublicKey::try_from(&private_key).unwrap().into();

    node.wallets.create(wallet);

    node.wallets
        .insert_adhoc2(&wallet, &private_key, false)
        .unwrap();

    node.wallets.work_set(&wallet, &public_key, 1).unwrap();

    let result = node
        .runtime
        .block_on(async { server.client.wallet_work_get(wallet).await.unwrap() });

    assert_eq!(
        result.works.get(&public_key.into()).unwrap(),
        &WorkNonce::from(1)
    );
}

#[test]
fn wallet_work_get_fails_without_enable_control() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let result = node
        .runtime
        .block_on(async { server.client.wallet_work_get(WalletId::zero()).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"RPC control is disabled\"".to_string())
    );
}

#[test]
fn wallet_work_get_fails_with_wallet_not_found() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node
        .runtime
        .block_on(async { server.client.wallet_work_get(WalletId::zero()).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet not found\"".to_string())
    );
}
