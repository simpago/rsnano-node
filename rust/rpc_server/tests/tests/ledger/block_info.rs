use rsnano_core::{Amount, BlockHash, BlockSubType};
use rsnano_ledger::{DEV_GENESIS, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use std::time::{SystemTime, UNIX_EPOCH};
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn block_info() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let result = node
        .runtime
        .block_on(async { rpc_client.block_info(*DEV_GENESIS_HASH).await.unwrap() });

    assert_eq!(result.amount, Amount::MAX);
    assert_eq!(result.balance, Amount::MAX);
    assert_eq!(result.block_account, *DEV_GENESIS_ACCOUNT);
    assert_eq!(result.confirmed, true);
    assert_eq!(result.height, 1);
    assert_eq!(result.subtype, BlockSubType::Open);
    assert_eq!(result.successor, BlockHash::zero());
    assert_eq!(result.contents, DEV_GENESIS.json_representation());

    let current_unix_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as u64;
    assert!(result.local_timestamp <= current_unix_timestamp);

    server.abort();
}
