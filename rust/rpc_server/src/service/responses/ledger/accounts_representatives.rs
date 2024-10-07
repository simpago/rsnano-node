use rsnano_core::Account;
use rsnano_node::node::Node;
use rsnano_rpc_messages::AccountsRepresentativesDto;
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn accounts_representatives(node: Arc<Node>, accounts: Vec<Account>) -> String {
    let tx = node.ledger.read_txn();
    let mut representatives: HashMap<Account, Account> = HashMap::new();
    let mut errors: HashMap<Account, String> = HashMap::new();

    for account in accounts {
        match node.ledger.store.account.get(&tx, &account) {
            Some(account_info) => {
                representatives.insert(account, account_info.representative.as_account());
            }
            None => {
                errors.insert(account, "Account not found".to_string());
            }
        }
    }

    let mut dto = AccountsRepresentativesDto::new(representatives);
    if !errors.is_empty() {
        dto.errors = Some(errors);
    }

    to_string_pretty(&dto).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_ledger::DEV_GENESIS_ACCOUNT;
    use rsnano_rpc_messages::AccountsRepresentativesDto;
    use std::collections::HashMap;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn accounts_representatives() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .accounts_representatives(vec![*DEV_GENESIS_ACCOUNT])
                .await
                .unwrap()
        });

        let mut accounts_representatives = HashMap::new();
        accounts_representatives.insert(*DEV_GENESIS_ACCOUNT, *DEV_GENESIS_ACCOUNT);

        let expected = AccountsRepresentativesDto::new(accounts_representatives);
        assert_eq!(result, expected);

        server.abort();
    }
}
