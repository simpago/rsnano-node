use rsnano_core::{RawKey, WalletId};
use rsnano_node::wallets::WalletsExt;
use rsnano_rpc_messages::WalletWithSeedArgs;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn wallet_change_seed() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::zero();
    node.wallets.create(wallet_id);
    let new_seed =
        RawKey::decode_hex("74F2B37AAD20F4A260F0A5B3CB3D7FB51673212263E58A380BC10474BB039CEE")
            .unwrap();

    node.runtime.block_on(async {
        server
            .client
            .wallet_change_seed(WalletWithSeedArgs::new(wallet_id, new_seed))
            .await
            .unwrap()
    });

    assert_eq!(node.wallets.get_seed(wallet_id).unwrap(), new_seed);
}

#[test]
fn wallet_change_seed_fails_without_enable_control() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async {
        server
            .client
            .wallet_change_seed(WalletWithSeedArgs::new(WalletId::zero(), RawKey::zero()))
            .await
    });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"RPC control is disabled\"".to_string())
    );
}
