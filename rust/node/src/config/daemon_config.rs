use super::{NodeConfig, NodeRpcConfig, OpenclConfig};
use crate::NetworkParams;
use anyhow::Result;

pub struct DaemonConfig {
    pub rpc_enable: bool,
    pub rpc: NodeRpcConfig,
    pub node: NodeConfig,
    pub opencl: OpenclConfig,
    pub opencl_enable: bool,
}

impl DaemonConfig {
    pub fn new(network_params: &NetworkParams, parallelism: usize) -> Result<Self> {
        Ok(Self {
            rpc_enable: false,
            node: NodeConfig::new(None, network_params, parallelism),
            opencl: OpenclConfig::new(),
            opencl_enable: false,
            rpc: NodeRpcConfig::new()?,
        })
    }
}
