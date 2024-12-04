use crate::command_handler::RpcCommandHandler;
use rsnano_node::wallets::WalletsExt;
use rsnano_rpc_messages::{WalletChangeSeedArgs, WalletChangeSeedResponse};

impl RpcCommandHandler {
    pub(crate) fn wallet_change_seed(
        &self,
        args: WalletChangeSeedArgs,
    ) -> WalletChangeSeedResponse {
        let (restored_count, last_restored_account) = self
            .node
            .wallets
            .change_seed(
                args.wallet,
                &args.seed,
                args.count.unwrap_or_default().inner(),
            )
            .unwrap();
        WalletChangeSeedResponse::new(last_restored_account, restored_count)
    }
}
