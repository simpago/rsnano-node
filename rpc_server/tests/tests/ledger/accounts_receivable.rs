use rsnano_core::{
    Account, Amount, Block, PublicKey, RawKey, StateBlock, WalletId, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_node::{wallets::WalletsExt, Node};
use rsnano_rpc_messages::{AccountsReceivableArgs, AccountsReceivableResponse};
use std::sync::Arc;
use std::time::Duration;
use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};

fn send_block(node: Arc<Node>, account: Account, amount: Amount) -> Block {
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

    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        previous,
        *DEV_GENESIS_PUB_KEY,
        balance - amount,
        account.into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(previous),
    ));

    node.process_active(send.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active_elections.active(&send),
        "not active on node",
    );

    send
}

#[test]
fn accounts_receivable_include_only_confirmed() {
    let mut system = System::new();
    let node = system.make_node();

    let wallet = WalletId::zero();
    node.wallets.create(wallet);
    let private_key = RawKey::zero();
    let public_key: PublicKey = (&private_key).try_into().unwrap();
    node.wallets
        .insert_adhoc2(&wallet, &private_key, false)
        .unwrap();

    let _send = send_block(node.clone(), public_key.into(), Amount::raw(1));

    let server = setup_rpc_client_and_server(node.clone(), false);

    let args = AccountsReceivableArgs::build(vec![public_key.into()])
        .count(1)
        .only_confirmed(true)
        .finish();

    let result1 = node
        .runtime
        .block_on(async { server.client.accounts_receivable(args).await.unwrap() });

    if let AccountsReceivableResponse::Simple(simple) = result1 {
        assert!(simple.blocks.is_empty());
    } else {
        panic!("Expected ReceivableDto::Blocks variant");
    }

    let args = AccountsReceivableArgs::build(vec![public_key.into()])
        .count(1)
        .finish();

    let result2 = node
        .runtime
        .block_on(async { server.client.accounts_receivable(args).await })
        .unwrap();

    if let AccountsReceivableResponse::Simple(simple) = result2 {
        assert!(simple.blocks.is_empty());
    } else {
        panic!("Expected ReceivableDto::Blocks variant");
    }
}

#[test]
fn accounts_receivable_options_none() {
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

    let server = setup_rpc_client_and_server(node.clone(), false);

    let args = AccountsReceivableArgs::build(vec![public_key.into()])
        .count(1)
        .only_confirmed(true)
        .finish();

    let result = node
        .runtime
        .block_on(async { server.client.accounts_receivable(args).await.unwrap() });

    if let AccountsReceivableResponse::Simple(simple) = result {
        assert_eq!(
            simple.blocks.get(&Account::from(public_key)).unwrap(),
            &vec![send.hash()]
        );
    } else {
        panic!("Expected ReceivableDto::Blocks variant");
    }
}

#[test]
fn accounts_receivable_threshold_some() {
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

    let server = setup_rpc_client_and_server(node.clone(), false);

    let args = AccountsReceivableArgs::build(vec![public_key.into()])
        .count(1)
        .threshold(Amount::raw(1))
        .finish();

    let result = node
        .runtime
        .block_on(async { server.client.accounts_receivable(args).await.unwrap() });

    if let AccountsReceivableResponse::Threshold(threshold) = result {
        assert_eq!(
            threshold
                .blocks
                .get(&Account::from(public_key))
                .unwrap()
                .get(&send2.hash())
                .unwrap(),
            &Amount::raw(2)
        );
    } else {
        panic!("Expected ReceivableDto::Threshold variant");
    }
}

#[test]
fn accounts_receivable_sorted() {
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

    let server = setup_rpc_client_and_server(node.clone(), false);

    let args = AccountsReceivableArgs::build(vec![public_key.into()])
        .count(1)
        .threshold(Amount::raw(1))
        .only_confirmed(false)
        .sorted()
        .finish();

    let result = node
        .runtime
        .block_on(async { server.client.accounts_receivable(args).await.unwrap() });

    if let AccountsReceivableResponse::Threshold(threshold) = result {
        assert_eq!(threshold.blocks.len(), 1);
        let (recv_account, recv_blocks) = threshold.blocks.iter().next().unwrap();
        assert_eq!(recv_account, &public_key.into());
        assert_eq!(recv_blocks.len(), 1);
        let (block_hash, amount) = recv_blocks.iter().next().unwrap();
        assert_eq!(block_hash, &send.hash());
        assert_eq!(amount, &Amount::raw(1));
    } else {
        panic!("Expected ReceivableDto::Threshold variant");
    }
}
