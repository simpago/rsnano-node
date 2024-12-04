use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{FrontiersResponse, WalletRpcMessage};
use std::collections::HashMap;

impl RpcCommandHandler {
    pub(crate) fn wallet_frontiers(
        &self,
        args: WalletRpcMessage,
    ) -> anyhow::Result<FrontiersResponse> {
        let tx = self.node.ledger.read_txn();
        let accounts = self.node.wallets.get_accounts_of_wallet(&args.wallet)?;
        let mut frontiers = HashMap::new();

        for account in accounts {
            if let Some(block_hash) = self.node.ledger.any().account_head(&tx, &account) {
                frontiers.insert(account, block_hash);
            }
        }
        Ok(FrontiersResponse::new(frontiers))
    }
}
