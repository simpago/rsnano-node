use rsnano_core::{Amount, BlockEnum, BlockHash, KeyPair, StateBlock, DEV_GENESIS_KEY};
use rsnano_ledger::{BlockStatus, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_node::Node;
use rsnano_rpc_messages::LedgerArgs;
use std::sync::Arc;
use test_helpers::{setup_rpc_client_and_server, System};

fn setup_test_environment(node: Arc<Node>) -> (KeyPair, BlockEnum, BlockEnum) {
    let keys = KeyPair::new();
    let rep_weight = Amount::MAX - Amount::raw(100);

    let send = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - rep_weight,
        keys.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let status = node.process_local(send.clone()).unwrap();
    assert_eq!(status, BlockStatus::Progress);

    let open = BlockEnum::State(StateBlock::new(
        keys.account(),
        BlockHash::zero(),
        *DEV_GENESIS_PUB_KEY,
        rep_weight,
        send.hash().into(),
        &keys,
        node.work_generate_dev(keys.public_key().into()),
    ));

    let status = node.process_local(open.clone()).unwrap();
    assert_eq!(status, BlockStatus::Progress);

    (keys, send, open)
}

#[test]
fn test_ledger() {
    let mut system = System::new();
    let node = system.build_node().finish();
    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let (keys, _, open) = setup_test_environment(node.clone());

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let args = LedgerArgs::builder().count(1).sorted().build();

    let result = node
        .runtime
        .block_on(async { rpc_client.ledger(args).await.unwrap() });

    let accounts = result.accounts;
    assert_eq!(accounts.len(), 1);

    for (account, info) in accounts {
        assert_eq!(keys.account(), account);
        assert_eq!(open.hash(), info.frontier);
        assert_eq!(open.hash(), info.open_block);
        assert_eq!(open.hash(), info.representative_block);
        assert_eq!(Amount::MAX - Amount::raw(100), info.balance);
        assert!(((time as i64) - (info.modified_timestamp as i64)).abs() < 5);
        assert_eq!(1, info.block_count);
        assert!(info.weight.is_none());
        assert!(info.pending.is_none());
        assert!(info.representative.is_none());
    }

    server.abort();
}

#[test]
fn test_ledger_threshold() {
    let mut system = System::new();
    let node = system.build_node().finish();
    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let (keys, _, _) = setup_test_environment(node.clone());

    let args = LedgerArgs::builder()
        .count(2)
        .sorted()
        .with_minimum_balance(Amount::MAX - Amount::raw(100))
        .build();

    let result = node
        .runtime
        .block_on(async { rpc_client.ledger(args).await.unwrap() });

    let accounts = result.accounts;
    assert_eq!(accounts.len(), 1);
    assert!(accounts.contains_key(&keys.account()));

    server.abort();
}

#[test]
fn test_ledger_pending() {
    let mut system = System::new();
    let node = system.build_node().finish();
    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let (keys, send_block, _) = setup_test_environment(node.clone());

    let send_amount = Amount::MAX - Amount::raw(100);
    let send2_amount = Amount::raw(50);
    let new_remaining_balance = Amount::MAX - send_amount - send2_amount;

    let send2_block = StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send_block.hash(),
        keys.account().into(),
        new_remaining_balance,
        keys.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send_block.hash().into()),
    );

    let status = node
        .process_local(BlockEnum::State(send2_block.clone()))
        .unwrap();
    assert_eq!(status, BlockStatus::Progress);

    let args = LedgerArgs::builder()
        .count(2)
        .include_receivables()
        .with_minimum_balance(Amount::MAX - Amount::raw(100))
        .build();

    let result = node
        .runtime
        .block_on(async { rpc_client.ledger(args).await.unwrap() });

    let accounts = result.accounts;
    assert_eq!(accounts.len(), 1);
    let account_info = accounts.get(&keys.account()).unwrap();
    assert_eq!(account_info.balance, send_amount);
    assert_eq!(account_info.pending, Some(send2_amount));

    server.abort();
}
