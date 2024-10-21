use rsnano_core::{
    Amount, BlockEnum, BlockHash, BlockSubType, KeyPair, StateBlock, WalletId, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_node::{wallets::WalletsExt, Node};
use std::sync::Arc;
use test_helpers::{setup_rpc_client_and_server, System};

fn setup_test_environment(node: Arc<Node>, keys: KeyPair, send_amount: Amount) -> BlockHash {
    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - send_amount,
        keys.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    node.process(send1.clone()).unwrap();

    let open_block = BlockEnum::State(StateBlock::new(
        keys.account(),
        BlockHash::zero(),
        keys.public_key(),
        send_amount,
        send1.hash().into(),
        &keys,
        node.work_generate_dev(keys.public_key().into()),
    ));

    node.process(open_block.clone()).unwrap();

    open_block.hash()
}

#[test]
fn wallet_history() {
    let mut system = System::new();
    let node = system.build_node().finish();
    let keys = KeyPair::new();
    let send_amount = Amount::from(100);
    let open_hash = setup_test_environment(node.clone(), keys.clone(), send_amount);

    let wallet_id = WalletId::zero();
    node.wallets.create(wallet_id);
    node.wallets
        .insert_adhoc2(&wallet_id, &keys.private_key(), true)
        .unwrap();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let wallet_history = node
        .runtime
        .block_on(async { rpc_client.wallet_history(wallet_id).await.unwrap() });

    assert_eq!(wallet_history.history.len(), 1);

    let entry = &wallet_history.history[0];

    assert_eq!(entry.entry_type, BlockSubType::Receive);
    assert_eq!(entry.account, *DEV_GENESIS_ACCOUNT);
    assert_eq!(entry.amount, send_amount);
    assert_eq!(entry.block_account, keys.account());
    assert_eq!(entry.hash, open_hash);

    // Assert that the timestamp is recent (within the last 10 seconds)
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    assert!(entry.local_timestamp <= current_time);
    assert!(entry.local_timestamp >= current_time - 10);

    server.abort();
}

#[test]
fn wallet_history_fails_with_wallet_not_found() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let result = node
        .runtime
        .block_on(async { rpc_client.wallet_history(WalletId::zero()).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Wallet not found\"".to_string())
    );

    server.abort();
}
