use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{LockedResponse, WalletRpcMessage};

impl RpcCommandHandler {
    pub(crate) fn wallet_locked(&self, args: WalletRpcMessage) -> anyhow::Result<LockedResponse> {
        let valid = self.node.wallets.valid_password(&args.wallet)?;
        Ok(LockedResponse::new(!valid))
    }
}
