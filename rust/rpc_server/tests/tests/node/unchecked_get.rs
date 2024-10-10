use rsnano_core::{Amount, BlockHash, JsonBlock, KeyPair, StateBlockBuilder};
use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};
use tokio::time::Duration;

#[test]
fn unchecked_get() {
    let mut system = System::new();
    let node = system.build_node().finish();
    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let key = KeyPair::new();

    let open = StateBlockBuilder::new()
        .account(key.account())
        .previous(BlockHash::zero())
        .representative(key.account())
        .balance(1)
        .link(key.account())
        .sign(&key)
        .work(node.work_generate_dev(key.account().into()))
        .build();

    node.process_active(open.clone());

    assert_timely_msg(
        Duration::from_secs(10),
        || node.unchecked.len() == 1,
        "Expected 1 unchecked block after 10 seconds",
    );

    let unchecked_dto = node
        .runtime
        .block_on(async { rpc_client.unchecked_get(open.hash()).await.unwrap() });

    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(unchecked_dto.modified_timestamp <= current_timestamp);

    let json_block: JsonBlock = unchecked_dto.contents;

    assert!(matches!(json_block, JsonBlock::State(_)));

    if let JsonBlock::State(state_block) = json_block {
        assert_eq!(state_block.account, key.account());
        assert_eq!(state_block.previous, BlockHash::zero());
        assert_eq!(state_block.representative, key.account());
        assert_eq!(state_block.balance, Amount::raw(1));
        assert_eq!(state_block.link, key.account().into());
    } else {
        panic!("Expected a state block");
    }

    server.abort();
}
