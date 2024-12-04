use crate::command_handler::RpcCommandHandler;
use rsnano_core::Account;
use rsnano_rpc_messages::{AccountsRepresentativesResponse, AccountsRpcMessage};
use std::collections::HashMap;

impl RpcCommandHandler {
    pub(crate) fn accounts_representatives(
        &self,
        args: AccountsRpcMessage,
    ) -> AccountsRepresentativesResponse {
        let tx = self.node.ledger.read_txn();
        let mut representatives: HashMap<Account, Account> = HashMap::new();
        let mut errors: HashMap<Account, String> = HashMap::new();

        for account in args.accounts {
            match self.node.ledger.store.account.get(&tx, &account) {
                Some(account_info) => {
                    representatives.insert(account, account_info.representative.as_account());
                }
                None => {
                    errors.insert(account, "Account not found".to_string());
                }
            }
        }

        AccountsRepresentativesResponse {
            representatives: if representatives.is_empty() {
                None
            } else {
                Some(representatives)
            },
            errors: if errors.is_empty() {
                None
            } else {
                Some(errors)
            },
        }
    }
}
