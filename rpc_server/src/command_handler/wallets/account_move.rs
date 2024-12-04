use crate::command_handler::RpcCommandHandler;
use rsnano_core::PublicKey;
use rsnano_rpc_messages::{AccountMoveArgs, MovedResponse};

impl RpcCommandHandler {
    pub(crate) fn account_move(&self, args: AccountMoveArgs) -> anyhow::Result<MovedResponse> {
        let public_keys: Vec<PublicKey> =
            args.accounts.iter().map(|account| account.into()).collect();

        self.node
            .wallets
            .move_accounts(&args.source, &args.wallet, &public_keys)?;

        Ok(MovedResponse::new(true))
    }
}
