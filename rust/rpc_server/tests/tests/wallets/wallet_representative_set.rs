use rsnano_core::{Account, PublicKey, WalletId};
use rsnano_node::wallets::WalletsExt;
use rsnano_rpc_messages::WalletRepresentativeSetArgs;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn wallet_representative_set() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet = WalletId::zero();
    node.wallets.create(wallet);

    node.runtime.block_on(async {
        rpc_client
            .wallet_representative_set(WalletRepresentativeSetArgs::new(wallet, Account::zero()))
            .await
            .unwrap()
    });

    assert_eq!(
        node.wallets.get_representative(wallet).unwrap(),
        PublicKey::zero()
    );

    server.abort();
}

#[test]
fn wallet_representative_set_fails_without_enable_control() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async {
        rpc_client
            .wallet_representative_set(WalletRepresentativeSetArgs::new(
                WalletId::zero(),
                Account::zero(),
            ))
            .await
    });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"RPC control is disabled\"".to_string())
    );

    server.abort();
}
