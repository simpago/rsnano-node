use rsnano_core::{Account, Amount};
use rsnano_node::node::Node;
use rsnano_rpc_messages::{AccountBalanceDto, AccountsBalancesDto};
use serde_json::to_string_pretty;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn accounts_balances(
    node: Arc<Node>,
    accounts: Vec<Account>,
    include_only_confirmed: Option<bool>,
) -> String {
    let tx = node.ledger.read_txn();
    let mut balances = HashMap::new();
    let only_confirmed = include_only_confirmed.unwrap_or(true);

    for account in accounts {
        let balance = if only_confirmed {
            node.ledger
                .confirmed()
                .account_balance(&tx, &account)
                .unwrap_or(Amount::zero())
        } else {
            node.ledger
                .any()
                .account_balance(&tx, &account)
                .unwrap_or(Amount::zero())
        };

        let pending = node
            .ledger
            .account_receivable(&tx, &account, only_confirmed);

        balances.insert(account, AccountBalanceDto::new(balance, pending, pending));
    }

    let accounts_balances = AccountsBalancesDto { balances };
    to_string_pretty(&accounts_balances).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Amount, BlockEnum, StateBlock, DEV_GENESIS_KEY};
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
    use rsnano_node::node::Node;
    use std::sync::Arc;
    use std::time::Duration;
    use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};

    fn send_block(node: Arc<Node>) {
        let send1 = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::raw(1),
            DEV_GENESIS_KEY.account().into(),
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
    fn accounts_balances_only_confirmed_none() {
        let mut system = System::new();
        let node = system.make_node();

        send_block(node.clone());

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
            rpc_client
                .accounts_balances(vec![DEV_GENESIS_KEY.public_key().as_account()], None)
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

        server.abort();
    }

    #[test]
    fn account_balance_only_confirmed_true() {
        let mut system = System::new();
        let node = system.make_node();

        send_block(node.clone());

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
            rpc_client
                .accounts_balances(vec![DEV_GENESIS_KEY.public_key().as_account()], Some(true))
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

        server.abort();
    }

    #[test]
    fn account_balance_only_confirmed_false() {
        let mut system = System::new();
        let node = system.make_node();

        send_block(node.clone());

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
            rpc_client
                .accounts_balances(vec![DEV_GENESIS_KEY.public_key().as_account()], Some(false))
                .await
                .unwrap()
        });

        let account = result.balances.get(&DEV_GENESIS_ACCOUNT).unwrap();

        assert_eq!(
            account.balance,
            Amount::raw(340282366920938463463374607431768211454)
        );

        assert_eq!(account.pending, Amount::raw(1));

        assert_eq!(account.receivable, Amount::raw(1));

        server.abort();
    }
}
