use rsnano_core::{Account, Amount};
use rsnano_node::Node;
use rsnano_rpc_messages::{AccountsWithAmountsDto, ErrorDto};
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn unopened(
    node: Arc<Node>,
    enable_control: bool,
    account: Account,
    count: u64,
    threshold: Option<Amount>,
) -> String {
    if !enable_control {
        return to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap();
    }

    let start = account;
    let mut accounts: HashMap<Account, Amount> = HashMap::new();

    let transaction = node.store.tx_begin_read();
    let mut iterator = node.store.pending.begin(&transaction);
    let end = node.store.pending.end();

    let mut current_account = start;
    let mut current_account_sum = Amount::zero();

    while iterator != end && accounts.len() < count as usize {
        let (key, info) = iterator.current().unwrap();
        let account = key.receiving_account;

        if node.store.account.get(&transaction, &account).is_some() {
            iterator = node.store.pending.begin_at_key(&transaction, key);
        } else {
            if account != current_account {
                if !current_account_sum.is_zero() {
                    if threshold.map_or(true, |t| current_account_sum >= t) {
                        accounts.insert(current_account, current_account_sum);
                    }
                    current_account_sum = Amount::zero();
                }
                current_account = account;
            }
            current_account_sum += info.amount;
        }
        iterator.next();
    }

    if accounts.len() < count as usize
        && !current_account_sum.is_zero()
        && threshold.map_or(true, |t| current_account_sum >= t)
    {
        accounts.insert(current_account, current_account_sum);
    }

    let response = AccountsWithAmountsDto::new("accounts".to_string(), accounts);

    to_string_pretty(&response).unwrap()
}
