use rsnano_core::{BlockHash, KeyPair, StateBlockBuilder};
use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};
use tokio::time::Duration;

#[test]
fn test_unchecked_keys() {
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
        Duration::from_secs(30),
        || {
            let len = node.unchecked.len();
            len == 1
        },
        "Expected 1 unchecked block after 30 seconds",
    );

    let open2 = StateBlockBuilder::new()
        .account(key.account())
        .previous(BlockHash::zero())
        .representative(key.account())
        .balance(2)
        .link(key.account())
        .sign(&key)
        .work(node.work_generate_dev(key.account().into()))
        .build();

    node.process_active(open2.clone());

    assert_timely_msg(
        Duration::from_secs(30),
        || {
            let len = node.unchecked.len();
            len == 2
        },
        "Expected 2 unchecked blocks after 30 seconds",
    );

    let unchecked_dto = node.runtime.block_on(async {
        rpc_client
            .unchecked_keys(key.account().into(), 2)
            .await
            .unwrap()
    });

    assert_eq!(
        unchecked_dto.unchecked.len(),
        2,
        "Expected 2 unchecked keys in DTO"
    );
    assert!(
        unchecked_dto
            .unchecked
            .iter()
            .any(|uk| uk.hash == open.hash()),
        "First hash not found in DTO"
    );
    assert!(
        unchecked_dto
            .unchecked
            .iter()
            .any(|uk| uk.hash == open2.hash()),
        "Second hash not found in DTO"
    );

    server.abort();
}
