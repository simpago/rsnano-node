use rsnano_core::{Account, Amount, BlockBuilder, JsonBlock, DEV_GENESIS_KEY};
use rsnano_ledger::DEV_GENESIS_HASH;
use rsnano_rpc_messages::ConfirmationInfoArgs;
use std::time::Duration;
use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};

#[test]
fn confirmation_info() {
    let mut system = System::new();
    let node = system.build_node().finish();

    let send = BlockBuilder::legacy_send()
        .previous(*DEV_GENESIS_HASH)
        .destination(Account::zero())
        .balance(Amount::MAX - Amount::raw(100))
        .sign((*DEV_GENESIS_KEY).clone())
        .work(node.work_generate_dev((*DEV_GENESIS_HASH).into()))
        .build();

    node.process_active(send.clone());

    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.active(&send),
        "not active on node 1",
    );

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let root = send.qualified_root();

    let args = ConfirmationInfoArgs::builder(root)
        .include_representatives()
        .build();

    let result = node
        .runtime
        .block_on(async { rpc_client.confirmation_info(args).await.unwrap() });

    //assert_eq!(result.announcements, 1); TODO
    assert_eq!(result.voters, 1);
    assert_eq!(result.last_winner, send.hash());

    let blocks = result.blocks;
    assert_eq!(blocks.len(), 1);

    let block = blocks.get(&send.hash()).unwrap();
    let representatives = block.representatives.clone().unwrap();
    assert_eq!(representatives.len(), 1);

    assert_eq!(result.total_tally, Amount::zero());

    let contents: &JsonBlock = block.contents.as_ref().unwrap();

    match contents {
        JsonBlock::Send(contents) => {
            assert_eq!(contents.previous, *DEV_GENESIS_HASH);
            assert_eq!(contents.destination, Account::zero());
            assert_eq!(
                Amount::from(contents.balance),
                Amount::MAX - Amount::raw(100)
            );
        }
        _ => (),
    }

    server.abort();
}
