use rsnano_core::{Amount, Block, StateBlock, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_node::Node;
use rsnano_rpc_messages::AccountsBalancesArgs;
use std::sync::Arc;
use std::time::Duration;
use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};

fn send_block(node: Arc<Node>) {
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        DEV_GENESIS_KEY.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    node.process_active(send1.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active_elections.active(&send1),
        "not active on node 1",
    );
}

#[test]
fn accounts_balances_only_confirmed_none() {
    let mut system = System::new();
    let node = system.make_node();

    send_block(node.clone());

    let server = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async {
        server
            .client
            .accounts_balances(vec![DEV_GENESIS_KEY.public_key().as_account()])
            .await
            .unwrap()
    });

    let account = result.balances.get(&DEV_GENESIS_ACCOUNT).unwrap();

    assert_eq!(
        account.balance,
        Amount::raw(340282366920938463463374607431768211455)
    );
    assert_eq!(account.pending, Amount::zero());
    assert_eq!(account.receivable, Amount::zero());
}

#[test]
fn account_balance_only_confirmed_true() {
    let mut system = System::new();
    let node = system.make_node();

    send_block(node.clone());

    let server = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async {
        server
            .client
            .accounts_balances(vec![DEV_GENESIS_KEY.public_key().as_account()])
            .await
            .unwrap()
    });

    let account = result.balances.get(&DEV_GENESIS_ACCOUNT).unwrap();

    assert_eq!(
        account.balance,
        Amount::raw(340282366920938463463374607431768211455)
    );

    assert_eq!(account.pending, Amount::zero());
    assert_eq!(account.receivable, Amount::zero());
}

#[test]
fn account_balance_only_confirmed_false() {
    let mut system = System::new();
    let node = system.make_node();

    send_block(node.clone());

    let server = setup_rpc_client_and_server(node.clone(), false);

    let args = AccountsBalancesArgs::new(vec![DEV_GENESIS_KEY.public_key().as_account()])
        .include_unconfirmed_blocks()
        .finish();

    let result = node
        .runtime
        .block_on(async { server.client.accounts_balances(args).await.unwrap() });

    let account = result.balances.get(&DEV_GENESIS_ACCOUNT).unwrap();

    assert_eq!(
        account.balance,
        Amount::raw(340282366920938463463374607431768211454)
    );

    assert_eq!(account.pending, Amount::raw(1));
    assert_eq!(account.receivable, Amount::raw(1));
}
