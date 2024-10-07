use rsnano_core::{Account, Amount};
use rsnano_node::node::Node;
use rsnano_rpc_messages::AccountBalanceDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_balance(
    node: Arc<Node>,
    account: Account,
    only_confirmed: Option<bool>,
) -> String {
    let tx = node.ledger.read_txn();
    let only_confirmed = only_confirmed.unwrap_or(true);

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

    let account_balance = AccountBalanceDto::new(balance, pending, pending);

    to_string_pretty(&account_balance).unwrap()
}

#[cfg(test)]
mod tests {
    use test_helpers::{send_block, setup_rpc_client_and_server};
    use rsnano_core::{Amount, DEV_GENESIS_KEY};
    use test_helpers::System;

    #[test]
    fn account_balance_only_confirmed_none() {
        let mut system = System::new();
        let node = system.make_node();

        send_block(node.clone());

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
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

        let result = node.tokio.block_on(async {
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

        let result = node.tokio.block_on(async {
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
}
