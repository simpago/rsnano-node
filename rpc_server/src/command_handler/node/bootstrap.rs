use crate::command_handler::RpcCommandHandler;
use anyhow::bail;
use rsnano_node::bootstrap::BootstrapInitiatorExt;
use rsnano_rpc_messages::{BootstrapArgs, SuccessResponse};
use std::net::SocketAddrV6;

impl RpcCommandHandler {
    pub(crate) fn bootstrap(&self, args: BootstrapArgs) -> anyhow::Result<SuccessResponse> {
        let bootstrap_id = args.id.unwrap_or(String::new());
        let endpoint = SocketAddrV6::new(args.address, args.port.into(), 0, 0);
        if self.node.flags.disable_legacy_bootstrap {
            bail!("Legacy bootstrap is disabled");
        }
        self.node.peer_connector.connect_to(endpoint);
        self.node
            .bootstrap_initiator
            .bootstrap2(endpoint, bootstrap_id);
        Ok(SuccessResponse::new())
    }
}
