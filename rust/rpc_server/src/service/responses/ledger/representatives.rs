use rsnano_core::{Account, Amount};
use rsnano_node::node::Node;
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

#[cfg(test)]
mod tests {
    use rsnano_core::Amount;
    use rsnano_ledger::DEV_GENESIS_ACCOUNT;
    use std::collections::HashMap;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn representatives_rpc_response() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.representatives(None, None).await.unwrap() });

        let mut representatives = HashMap::new();
        representatives.insert(*DEV_GENESIS_ACCOUNT, Amount::MAX);

        assert_eq!(result.value, representatives);

        server.abort();
    }
}
