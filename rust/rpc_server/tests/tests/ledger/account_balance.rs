use rsnano_core::{Amount, DEV_GENESIS_KEY};
use test_helpers::{send_block, setup_rpc_client_and_server, System};

#[test]
fn account_balance_only_confirmed_none() {
    let mut system = System::new();
    let node = system.make_node();

    send_block(node.clone());

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async {
        rpc_client
            .account_balance(DEV_GENESIS_KEY.public_key().as_account(), None)
            .await
            .unwrap()
    });

    assert_eq!(
        result.balance,
        Amount::raw(340282366920938463463374607431768211455)
    );

    assert_eq!(result.pending, Amount::zero());

    assert_eq!(result.receivable, Amount::zero());

    server.abort();
}

#[test]
fn account_balance_only_confirmed_true() {
    let mut system = System::new();
    let node = system.make_node();

    send_block(node.clone());

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async {
        rpc_client
            .account_balance(DEV_GENESIS_KEY.public_key().as_account(), Some(true))
            .await
            .unwrap()
    });

    assert_eq!(
        result.balance,
        Amount::raw(340282366920938463463374607431768211455)
    );

    assert_eq!(result.pending, Amount::zero());

    assert_eq!(result.receivable, Amount::zero());

    server.abort();
}

#[test]
fn account_balance_only_confirmed_false() {
    let mut system = System::new();
    let node = system.make_node();

    send_block(node.clone());

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async {
        rpc_client
            .account_balance(DEV_GENESIS_KEY.public_key().as_account(), Some(false))
            .await
            .unwrap()
    });

    assert_eq!(
        result.balance,
        Amount::raw(340282366920938463463374607431768211454)
    );

    assert_eq!(result.pending, Amount::raw(1));

    assert_eq!(result.receivable, Amount::raw(1));

    server.abort();
}
