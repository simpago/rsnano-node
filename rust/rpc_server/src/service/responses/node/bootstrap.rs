use rsnano_node::{bootstrap::BootstrapInitiatorExt, Node};
use rsnano_rpc_messages::SuccessDto;
use serde_json::to_string_pretty;
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::Arc,
};

pub async fn bootstrap(
    node: Arc<Node>,
    address: Ipv6Addr,
    port: u16,
    id: Option<String>,
) -> String {
    let id = id.unwrap_or(String::new());
    let endpoint = SocketAddrV6::new(address, port, 0, 0);
    node.bootstrap_initiator.bootstrap2(endpoint, id);

    to_string_pretty(&SuccessDto::new()).unwrap()
}
