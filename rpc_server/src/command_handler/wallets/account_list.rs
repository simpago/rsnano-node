use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{AccountsRpcMessage, WalletRpcMessage};

impl RpcCommandHandler {
    pub(crate) fn account_list(
        &self,
        args: WalletRpcMessage,
    ) -> anyhow::Result<AccountsRpcMessage> {
        let accounts = self.node.wallets.get_accounts_of_wallet(&args.wallet)?;
        Ok(AccountsRpcMessage::new(accounts))
    }
}
