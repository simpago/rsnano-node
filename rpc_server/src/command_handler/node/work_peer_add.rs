use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{AddressWithPortArgs, RpcError};
use tracing::warn;

impl RpcCommandHandler {
    pub(crate) fn work_peer_add(&self, _args: AddressWithPortArgs) -> RpcError {
        warn!("Distributed work feature is not implemented yet");
        RpcError::new(Self::NOT_IMPLEMENTED)
    }
}
