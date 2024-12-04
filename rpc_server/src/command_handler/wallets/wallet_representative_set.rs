use crate::command_handler::RpcCommandHandler;
use rsnano_node::wallets::WalletsExt;
use rsnano_rpc_messages::{SetResponse, WalletRepresentativeSetArgs};

impl RpcCommandHandler {
    pub(crate) fn wallet_representative_set(
        &self,
        args: WalletRepresentativeSetArgs,
    ) -> anyhow::Result<SetResponse> {
        let update_existing = args.update_existing_accounts.unwrap_or_default().inner();
        self.node.wallets.set_representative(
            args.wallet,
            args.representative.into(),
            update_existing,
        )?;
        Ok(SetResponse::new(true))
    }
}
