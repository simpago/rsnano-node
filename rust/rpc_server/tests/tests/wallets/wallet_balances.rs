use rsnano_core::{
    Account, Amount, BlockEnum, PublicKey, RawKey, StateBlock, WalletId, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_node::{wallets::WalletsExt, Node};
use rsnano_rpc_messages::{AccountBalanceDto, AccountsBalancesDto};
use std::{collections::HashMap, sync::Arc, time::Duration};
use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};

fn send_block(node: Arc<Node>, account: Account) {
    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        account.into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    node.process_active(send1.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.active(&send1),
        "not active on node 1",
    );
}

#[test]
fn wallet_balances_threshold_none() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    node.wallets.create(1.into());

    let result = node
        .runtime
        .block_on(async { rpc_client.wallet_balances(1.into(), None).await.unwrap() });

    let expected_balances: HashMap<Account, AccountBalanceDto> = HashMap::new();
    let expected_result = AccountsBalancesDto {
        balances: expected_balances,
    };

    assert_eq!(result, expected_result);

    server.abort();
}

#[test]
fn wallet_balances_threshold_some() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let wallet: WalletId = 1.into();
    let private_key = RawKey::zero();
    let public_key: PublicKey = (&private_key).try_into().unwrap();

    node.wallets.create(wallet);

    node.wallets
        .insert_adhoc2(&wallet, &RawKey::zero(), false)
        .unwrap();

    send_block(node.clone(), public_key.into());

    let result = node.runtime.block_on(async {
        rpc_client
            .wallet_balances(wallet, Some(Amount::zero()))
            .await
            .unwrap()
    });

    let mut expected_balances: HashMap<Account, AccountBalanceDto> = HashMap::new();
    expected_balances.insert(
        public_key.into(),
        AccountBalanceDto::new(Amount::zero(), Amount::raw(1), Amount::raw(1)),
    );
    let expected_result = AccountsBalancesDto {
        balances: expected_balances,
    };

    assert_eq!(result, expected_result);

    server.abort();
}

#[test]
fn wallet_balances_threshold_some_fails() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    node.wallets.create(1.into());

    let public_key = node
        .wallets
        .deterministic_insert2(&1.into(), false)
        .unwrap();

    send_block(node.clone(), public_key.into());

    let result = node.runtime.block_on(async {
        rpc_client
            .wallet_balances(1.into(), Some(Amount::raw(2)))
            .await
            .unwrap()
    });

    let expected_balances: HashMap<Account, AccountBalanceDto> = HashMap::new();
    let expected_result = AccountsBalancesDto {
        balances: expected_balances,
    };

    assert_eq!(result, expected_result);

    server.abort();
}
