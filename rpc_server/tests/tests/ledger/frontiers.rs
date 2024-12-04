use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn frontiers() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node.runtime.block_on(async {
        server
            .client
            .frontiers(*DEV_GENESIS_ACCOUNT, 1)
            .await
            .unwrap()
    });

    assert_eq!(
        result
            .frontiers
            .unwrap()
            .get(&*DEV_GENESIS_ACCOUNT)
            .unwrap(),
        &*DEV_GENESIS_HASH
    );
}
