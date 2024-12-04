use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{WalletRepresentativeResponse, WalletRpcMessage};

impl RpcCommandHandler {
    pub(crate) fn wallet_representative(
        &self,
        args: WalletRpcMessage,
    ) -> anyhow::Result<WalletRepresentativeResponse> {
        let representative = self.node.wallets.get_representative(args.wallet)?;
        Ok(WalletRepresentativeResponse::new(representative.into()))
    }
}
