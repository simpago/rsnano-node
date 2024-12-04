use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{JsonResponse, WalletRpcMessage};

impl RpcCommandHandler {
    pub(crate) fn wallet_export(&self, args: WalletRpcMessage) -> anyhow::Result<JsonResponse> {
        let json = self.node.wallets.serialize(args.wallet)?;
        Ok(JsonResponse::new(json))
    }
}
