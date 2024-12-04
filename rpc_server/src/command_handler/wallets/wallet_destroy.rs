use crate::command_handler::RpcCommandHandler;
use anyhow::bail;
use rsnano_rpc_messages::{DestroyedResponse, WalletRpcMessage};

impl RpcCommandHandler {
    pub(crate) fn wallet_destroy(
        &self,
        args: WalletRpcMessage,
    ) -> anyhow::Result<DestroyedResponse> {
        if !self.node.wallets.wallet_exists(&args.wallet) {
            bail!("Wallet not found");
        }
        self.node.wallets.destroy(&args.wallet);
        let destroyed = !self.node.wallets.wallet_exists(&args.wallet);
        Ok(DestroyedResponse::new(destroyed))
    }
}
