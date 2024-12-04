use rsnano_ledger::DEV_GENESIS_HASH;
use rsnano_rpc_messages::WorkValidateArgs;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn work_validate() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);
    let work = node.work_generate_dev(*DEV_GENESIS_HASH);

    let result = node.runtime.block_on(async {
        server
            .client
            .work_validate(WorkValidateArgs {
                work: Some(1.into()),
                hash: *DEV_GENESIS_HASH,
                multiplier: None,
                difficulty: None,
            })
            .await
            .unwrap()
    });

    assert_eq!(result.valid_all, "0");
    assert_eq!(result.valid_receive, "0");

    let result = node.runtime.block_on(async {
        server
            .client
            .work_validate(WorkValidateArgs {
                work: Some(work.into()),
                hash: *DEV_GENESIS_HASH,
                multiplier: None,
                difficulty: None,
            })
            .await
            .unwrap()
    });

    assert_eq!(result.valid_all, "1");
    assert_eq!(result.valid_receive, "1");
}
