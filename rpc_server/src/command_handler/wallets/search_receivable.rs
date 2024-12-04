use crate::command_handler::RpcCommandHandler;
use rsnano_node::wallets::{WalletsError, WalletsExt};
use rsnano_rpc_messages::{StartedResponse, WalletRpcMessage};

impl RpcCommandHandler {
    pub(crate) fn search_receivable(
        &self,
        args: WalletRpcMessage,
    ) -> anyhow::Result<StartedResponse> {
        match self.node.wallets.search_receivable_wallet(args.wallet) {
            Ok(_) => Ok(StartedResponse::new(true)),
            Err(WalletsError::WalletLocked) => Ok(StartedResponse::new(false)),
            Err(e) => Err(e.into()),
        }
    }
}
