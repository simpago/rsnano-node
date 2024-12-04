use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{ExistsResponse, WalletWithAccountArgs};

impl RpcCommandHandler {
    pub(crate) fn wallet_contains(
        &self,
        args: WalletWithAccountArgs,
    ) -> anyhow::Result<ExistsResponse> {
        let wallet_accounts = self.node.wallets.get_accounts_of_wallet(&args.wallet)?;
        let exists = wallet_accounts.contains(&args.account);
        Ok(ExistsResponse::new(exists))
    }
}
