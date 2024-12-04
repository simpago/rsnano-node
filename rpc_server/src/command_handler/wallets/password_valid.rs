use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{ValidResponse, WalletRpcMessage};

impl RpcCommandHandler {
    pub(crate) fn password_valid(&self, args: WalletRpcMessage) -> anyhow::Result<ValidResponse> {
        let valid = self.node.wallets.valid_password(&args.wallet)?;
        Ok(ValidResponse::new(valid))
    }
}
