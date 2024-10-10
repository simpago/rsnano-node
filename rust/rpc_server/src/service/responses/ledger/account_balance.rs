use rsnano_core::{Account, Amount};
use rsnano_node::Node;
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
