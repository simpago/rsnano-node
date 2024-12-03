use crate::command_handler::RpcCommandHandler;
use rsnano_node::{BUILD_INFO, VERSION_STRING};
use rsnano_rpc_messages::VersionResponse;

impl RpcCommandHandler {
    pub(crate) fn version(&self) -> VersionResponse {
        let tx = self.node.ledger.read_txn();
        VersionResponse {
            rpc_version: 1.into(),
            store_version: (self.node.store.version.get(&tx).unwrap_or_default() as u32).into(),
            protocol_version: self.node.network_params.network.protocol_version.into(),
            node_vendor: format!("RsNano {}", VERSION_STRING),
            store_vendor: self.node.store.vendor(),
            network: self
                .node
                .network_params
                .network
                .current_network
                .as_str()
                .to_owned(),
            network_identifier: self.node.network_params.ledger.genesis_block.hash(),
            build_info: BUILD_INFO.to_owned(),
        }
    }
}
