use rsnano_core::{WalletId, DEV_GENESIS_KEY};
use rsnano_ledger::DEV_GENESIS_ACCOUNT;
use rsnano_node::wallets::WalletsExt;
use test_helpers::{send_block, setup_rpc_client_and_server, System};

#[test]
fn wallet_frontiers() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet = WalletId::zero();

    node.wallets.create(wallet);
    node.wallets
        .insert_adhoc2(&wallet, &DEV_GENESIS_KEY.private_key(), false)
        .unwrap();

    let hash = send_block(node.clone());

    let result = node
        .runtime
        .block_on(async { rpc_client.wallet_frontiers(wallet).await.unwrap() });

    assert_eq!(result.frontiers.get(&*DEV_GENESIS_ACCOUNT).unwrap(), &hash);

    server.abort();
}
