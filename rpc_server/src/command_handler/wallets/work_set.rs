use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{SuccessResponse, WorkSetArgs};

impl RpcCommandHandler {
    pub(crate) fn work_set(&self, args: WorkSetArgs) -> anyhow::Result<SuccessResponse> {
        self.node
            .wallets
            .work_set(&args.wallet, &args.account.into(), args.work.into())?;
        Ok(SuccessResponse::new())
    }
}
