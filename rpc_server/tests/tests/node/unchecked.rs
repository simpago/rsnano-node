use rsnano_core::{BlockHash, PrivateKey, StateBlockBuilder};
use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};
use tokio::time::Duration;

#[test]
fn test_unchecked() {
    let mut system = System::new();
    let node = system.build_node().finish();
    let server = setup_rpc_client_and_server(node.clone(), true);

    let key = PrivateKey::new();

    let open = StateBlockBuilder::new()
        .account(key.account())
        .previous(BlockHash::zero())
        .representative(key.account())
        .balance(1)
        .link(key.account())
        .sign(&key)
        .work(node.work_generate_dev(key.account()))
        .build();

    let open2 = StateBlockBuilder::new()
        .account(key.account())
        .previous(BlockHash::zero())
        .representative(key.account())
        .balance(2)
        .link(key.account())
        .sign(&key)
        .work(node.work_generate_dev(key.account()))
        .build();

    node.process_active(open.clone());

    node.process_active(open2.clone());

    assert_timely_msg(
        Duration::from_secs(10),
        || node.unchecked.len() == 2,
        "Expected 2 unchecked blocks after 10 seconds",
    );

    let unchecked_dto = node
        .runtime
        .block_on(async { server.client.unchecked(2).await.unwrap() });

    assert_eq!(unchecked_dto.blocks.len(), 2);
    assert!(unchecked_dto.blocks.contains_key(&open.hash()));
    assert!(unchecked_dto.blocks.contains_key(&open2.hash()));
}
