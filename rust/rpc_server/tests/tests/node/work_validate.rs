use rsnano_ledger::DEV_GENESIS_HASH;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn work_validate() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let work = node.work_generate_dev((*DEV_GENESIS_HASH).into());

    let result = node.runtime.block_on(async {
        rpc_client
            .work_validate(1.into(), *DEV_GENESIS_HASH)
            .await
            .unwrap()
    });

    assert_eq!(result.valid_all, false);
    assert_eq!(result.valid_receive, false);

    let result = node.runtime.block_on(async {
        rpc_client
            .work_validate(work.into(), *DEV_GENESIS_HASH)
            .await
            .unwrap()
    });

    assert_eq!(result.valid_all, true);
    assert_eq!(result.valid_receive, true);

    server.abort();
}
