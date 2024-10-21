use rsnano_core::{
    Account, Amount, BlockEnum, PublicKey, RawKey, StateBlock, WalletId, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_node::{wallets::WalletsExt, Node};
use rsnano_rpc_messages::{ReceivableArgs, ReceivableDto};
use std::sync::Arc;
use std::time::Duration;
use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};

fn send_block(node: Arc<Node>, account: Account, amount: Amount) -> BlockEnum {
    let transaction = node.ledger.read_txn();
    let previous = node
        .ledger
        .any()
        .account_head(&transaction, &*DEV_GENESIS_ACCOUNT)
        .unwrap_or(*DEV_GENESIS_HASH);
    let balance = node
        .ledger
        .any()
        .account_balance(&transaction, &*DEV_GENESIS_ACCOUNT)
        .unwrap_or(Amount::MAX);

    let send = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        previous,
        *DEV_GENESIS_PUB_KEY,
        balance - amount,
        account.into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(previous.into()),
    ));

    node.process_active(send.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.active(&send),
        "not active on node",
    );

    send
}

#[test]
fn receivable_include_only_confirmed() {
    let mut system = System::new();
    let node = system.make_node();

    let wallet = WalletId::zero();
    node.wallets.create(wallet);
    let private_key = RawKey::zero();
    let public_key: PublicKey = (&private_key).try_into().unwrap();
    node.wallets
        .insert_adhoc2(&wallet, &private_key, false)
        .unwrap();

    let send = send_block(node.clone(), public_key.into(), Amount::raw(1));

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let result1 = node.runtime.block_on(async {
        rpc_client
            .receivable(ReceivableArgs::new(public_key.into(), 1))
            .await
            .unwrap()
    });

    if let ReceivableDto::Blocks { blocks } = result1 {
        assert!(blocks.get(&public_key.into()).unwrap().is_empty());
    } else {
        panic!("Expected ReceivableDto::Blocks variant");
    }

    let args = ReceivableArgs::builder(public_key.into(), 1)
        .include_unconfirmed_blocks()
        .build();

    let result2 = node
        .runtime
        .block_on(async { rpc_client.receivable(args).await.unwrap() });

    if let ReceivableDto::Blocks { blocks } = result2 {
        assert_eq!(blocks.get(&public_key.into()).unwrap(), &vec![send.hash()]);
    } else {
        panic!("Expected ReceivableDto::Blocks variant");
    }

    server.abort();
}

#[test]
fn receivable_options_none() {
    let mut system = System::new();
    let node = system.make_node();

    let wallet = WalletId::zero();
    node.wallets.create(wallet);
    let private_key = RawKey::zero();
    let public_key: PublicKey = (&private_key).try_into().unwrap();
    node.wallets
        .insert_adhoc2(&wallet, &private_key, false)
        .unwrap();

    let send = send_block(node.clone(), public_key.into(), Amount::raw(1));
    node.ledger.confirm(&mut node.ledger.rw_txn(), send.hash());

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async {
        rpc_client
            .receivable(ReceivableArgs::new(public_key.into(), 1))
            .await
            .unwrap()
    });

    if let ReceivableDto::Blocks { blocks } = result {
        assert_eq!(blocks.get(&public_key.into()).unwrap(), &vec![send.hash()]);
    } else {
        panic!("Expected ReceivableDto::Blocks variant");
    }

    server.abort();
}

#[test]
fn receivable_threshold_some() {
    let mut system = System::new();
    let node = system.make_node();

    let wallet = WalletId::zero();
    node.wallets.create(wallet);
    let private_key = RawKey::zero();
    let public_key: PublicKey = (&private_key).try_into().unwrap();
    node.wallets
        .insert_adhoc2(&wallet, &private_key, false)
        .unwrap();

    let send = send_block(node.clone(), public_key.into(), Amount::raw(1));
    node.ledger.confirm(&mut node.ledger.rw_txn(), send.hash());
    let send2 = send_block(node.clone(), public_key.into(), Amount::raw(2));
    node.ledger.confirm(&mut node.ledger.rw_txn(), send2.hash());

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let args = ReceivableArgs::builder(public_key.into(), 2)
        .threshold(Amount::raw(1))
        .build();

    let result = node
        .runtime
        .block_on(async { rpc_client.receivable(args).await.unwrap() });

    println!("{:?}", result);

    if let ReceivableDto::Threshold { blocks } = result {
        assert_eq!(
            blocks
                .get(&public_key.into())
                .unwrap()
                .get(&send2.hash())
                .unwrap(),
            &Amount::raw(2)
        );
    } else {
        panic!("Expected ReceivableDto::Threshold variant");
    }

    server.abort();
}

#[test]
fn receivable_sorting() {
    let mut system = System::new();
    let node = system.make_node();

    let wallet = WalletId::zero();
    node.wallets.create(wallet);
    let private_key = RawKey::zero();
    let public_key: PublicKey = (&private_key).try_into().unwrap();
    node.wallets
        .insert_adhoc2(&wallet, &private_key, false)
        .unwrap();

    let send = send_block(node.clone(), public_key.into(), Amount::raw(1));

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let args = ReceivableArgs::builder(public_key.into(), 1)
        .sorting()
        .include_unconfirmed_blocks()
        .build();

    let result = node
        .runtime
        .block_on(async { rpc_client.receivable(args).await.unwrap() });

    if let ReceivableDto::Blocks { blocks } = result {
        assert_eq!(blocks.len(), 1);
        let recv_blocks = blocks.get(&public_key.into()).unwrap();
        assert_eq!(recv_blocks.len(), 1);
        assert_eq!(recv_blocks[0], send.hash());
    } else {
        panic!("Expected ReceivableDto::Blocks variant");
    }

    server.abort();
}
