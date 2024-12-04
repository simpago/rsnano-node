use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{SuccessResponse, WalletAddWatchArgs};

impl RpcCommandHandler {
    pub(crate) fn wallet_add_watch(
        &self,
        args: WalletAddWatchArgs,
    ) -> anyhow::Result<SuccessResponse> {
        self.node
            .wallets
            .insert_watch(&args.wallet, &args.accounts)?;
        Ok(SuccessResponse::new())
    }
}
