use crate::command_handler::RpcCommandHandler;
use rsnano_node::wallets::{WalletsError, WalletsExt};
use rsnano_rpc_messages::{ValidResponse, WalletWithPasswordArgs};

impl RpcCommandHandler {
    pub(crate) fn password_enter(
        &self,
        args: WalletWithPasswordArgs,
    ) -> anyhow::Result<ValidResponse> {
        match self
            .node
            .wallets
            .enter_password(args.wallet, &args.password)
        {
            Ok(_) => Ok(ValidResponse::new(true)),
            Err(WalletsError::InvalidPassword) => Ok(ValidResponse::new(false)),
            Err(e) => Err(e.into()),
        }
    }
}
