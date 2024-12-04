use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{HashRpcMessage, SuccessResponse};

impl RpcCommandHandler {
    pub(crate) fn work_cancel(&self, args: HashRpcMessage) -> SuccessResponse {
        self.node.distributed_work.cancel(args.hash.into());
        SuccessResponse::new()
    }
}
