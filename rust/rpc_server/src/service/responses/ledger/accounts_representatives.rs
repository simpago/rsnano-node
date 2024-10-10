use rsnano_core::Account;
use rsnano_node::Node;
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
