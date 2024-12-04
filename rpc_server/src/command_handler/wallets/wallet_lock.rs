use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{LockedResponse, WalletRpcMessage};

impl RpcCommandHandler {
    pub(crate) fn wallet_lock(&self, args: WalletRpcMessage) -> anyhow::Result<LockedResponse> {
        self.node.wallets.lock(&args.wallet)?;
        Ok(LockedResponse::new(true))
    }
}
