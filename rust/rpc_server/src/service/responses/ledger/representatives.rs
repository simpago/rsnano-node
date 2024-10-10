use rsnano_core::{Account, Amount};
use rsnano_node::Node;
use rsnano_rpc_messages::AccountsWithAmountsDto;
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn representatives(node: Arc<Node>, count: Option<u64>, sorting: Option<bool>) -> String {
    let mut representatives: Vec<(Account, Amount)> = node
        .ledger
        .rep_weights
        .read()
        .iter()
        .map(|(pk, amount)| (Account::from(pk), *amount))
        .collect();

    if sorting.unwrap_or(false) {
        representatives.sort_by(|a, b| b.1.cmp(&a.1));
    }

    let count = count.unwrap_or(std::u64::MAX);
    let limited_representatives: HashMap<Account, Amount> =
        representatives.into_iter().take(count as usize).collect();

    to_string_pretty(&AccountsWithAmountsDto::new(
        "representatives".to_string(),
        limited_representatives,
    ))
    .unwrap()
}
