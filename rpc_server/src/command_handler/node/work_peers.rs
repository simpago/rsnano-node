use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::RpcError;
use tracing::warn;

impl RpcCommandHandler {
    pub(crate) fn work_peers(&self) -> RpcError {
        warn!("Distributed work feature is not implemented yet");
        RpcError::new(Self::NOT_IMPLEMENTED)
    }
}
