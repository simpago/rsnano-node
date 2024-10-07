use rsnano_core::{Account, Amount};
use rsnano_node::node::Node;
use rsnano_rpc_messages::{AccountsWithAmountsDto, DelegatorsArgs};
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn delegators(node: Arc<Node>, args: DelegatorsArgs) -> String {
    let representative = args.account;
    let count = args.count.unwrap_or(1024);
    let threshold = args.threshold.unwrap_or(Amount::zero());
    let start_account = args.start.unwrap_or(Account::zero());

    let mut delegators: HashMap<Account, Amount> = HashMap::new();
    let tx = node.ledger.read_txn();
    let mut iter = node.store.account.begin_account(&tx, &start_account);

    while let Some((account, info)) = iter.current() {
        if delegators.len() >= count as usize {
            break;
        }

        if info.representative == representative.into() && info.balance >= threshold {
            delegators.insert(*account, info.balance);
        }

        iter.next();
    }
    to_string_pretty(&AccountsWithAmountsDto::new(
        "delegators".to_string(),
        delegators,
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
    fn delegators_rpc_response() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .delegators(*DEV_GENESIS_ACCOUNT, None, None, None)
                .await
                .unwrap()
        });

        let mut delegators = HashMap::new();
        delegators.insert(*DEV_GENESIS_ACCOUNT, Amount::MAX);

        assert_eq!(result.value, delegators);

        server.abort();
    }
}
