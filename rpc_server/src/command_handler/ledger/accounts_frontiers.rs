use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{AccountsRpcMessage, FrontiersResponse};
use std::collections::HashMap;

impl RpcCommandHandler {
    pub(crate) fn accounts_frontiers(&self, args: AccountsRpcMessage) -> FrontiersResponse {
        let tx = self.node.ledger.read_txn();
        let mut frontiers = HashMap::new();
        let mut errors = HashMap::new();

        for account in args.accounts {
            if let Some(block_hash) = self.node.ledger.any().account_head(&tx, &account) {
                frontiers.insert(account, block_hash);
            } else {
                errors.insert(account, "Account not found".to_string());
            }
        }

        let mut frontiers = FrontiersResponse::new(frontiers);
        if !errors.is_empty() {
            frontiers.errors = Some(errors);
        }

        frontiers
    }
}
