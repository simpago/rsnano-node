use rsnano_core::{Amount, KeyPair, WalletId, DEV_GENESIS_KEY};
use rsnano_ledger::DEV_GENESIS_ACCOUNT;
use rsnano_node::wallets::WalletsExt;
use rsnano_rpc_messages::ChainArgs;
use std::{time::Duration, u64};
use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};

#[test]
fn successors() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let wallet_id = WalletId::zero();
    node.wallets.create(wallet_id);
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    let genesis = node.latest(&*DEV_GENESIS_ACCOUNT);
    assert!(!genesis.is_zero());

    let key = KeyPair::new();
    let block = node
        .wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key.account(),
            Amount::raw(1),
            0,
            true,
            None,
        )
        .unwrap();

    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.active(&block),
        "block not active on node",
    );

    let result = node.runtime.block_on(async {
        server
            .client
            .successors(ChainArgs::builder(genesis, u64::MAX).build())
            .await
            .unwrap()
    });

    let blocks = result.blocks.clone();

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0], genesis);
    assert_eq!(blocks[1], block.hash());

    let args = ChainArgs::builder(genesis, u64::MAX).reverse().build();

    let reverse_result = node
        .runtime
        .block_on(async { server.client.chain(args).await.unwrap() });

    assert_eq!(result, reverse_result);
}
