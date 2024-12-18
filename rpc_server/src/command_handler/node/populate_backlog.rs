use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::SuccessResponse;

impl RpcCommandHandler {
    pub(crate) fn populate_backlog(&self) -> SuccessResponse {
        self.node.backlog_scan.trigger();
        SuccessResponse::new()
    }
}
