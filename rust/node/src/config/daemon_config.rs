use super::{NodeConfig, NodeRpcConfig, OpenclConfig};
use crate::NetworkParams;

#[derive(Debug, PartialEq)]
pub struct DaemonConfig {
    pub rpc_enable: bool,
    pub rpc: NodeRpcConfig,
    pub node: NodeConfig,
    pub opencl: OpenclConfig,
    pub opencl_enable: bool,
}

impl DaemonConfig {
    pub fn new(network_params: &NetworkParams, parallelism: usize) -> Self {
        Self {
            rpc_enable: false,
            node: NodeConfig::new(
                Some(network_params.network.default_node_port),
                network_params,
                parallelism,
            ),
            opencl: OpenclConfig::new(),
            opencl_enable: false,
            rpc: NodeRpcConfig::new(),
        }
    }
}
