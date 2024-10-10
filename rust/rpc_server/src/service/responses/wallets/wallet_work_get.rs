use rsnano_core::WalletId;
use rsnano_node::Node;
use rsnano_rpc_messages::{AccountsWithWorkDto, ErrorDto};
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn wallet_work_get(node: Arc<Node>, enable_control: bool, wallet: WalletId) -> String {
    if !enable_control {
        return to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap();
    }

    let accounts = match node.wallets.get_accounts_of_wallet(&wallet) {
        Ok(accounts) => accounts,
        Err(e) => return to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    };

    let mut works = HashMap::new();

    for account in accounts {
        match node.wallets.work_get2(&wallet, &account.into()) {
            Ok(work) => {
                works.insert(account, work.into());
            }
            Err(e) => return to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    }

    to_string_pretty(&AccountsWithWorkDto::new(works)).unwrap()
}
