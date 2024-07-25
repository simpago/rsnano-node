use super::{NodeConfig, NodeRpcConfig, OpenclConfig};
use crate::NetworkParams;
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use toml::Value;

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

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_child("rpc", &mut |rpc| {
            self.rpc.serialize_toml(rpc)?;
            rpc.put_bool(
                "enable",
                self.rpc_enable,
                "Enable or disable RPC\ntype:bool",
            )?;
            Ok(())
        })?;

        toml.put_child("node", &mut |node| self.node.serialize_toml(node))?;

        toml.put_child("opencl", &mut |opencl| {
            self.opencl.serialize_toml(opencl)?;
            opencl.put_bool(
                "enable",
                self.opencl_enable,
                "Enable or disable OpenCL work generation\nIf enabled, consider freeing up CPU resources by setting [work_threads] to zero\ntype:bool",
            )?;
            Ok(())
        })?;

        Ok(())
    }

    pub fn deserialize_toml(&mut self, toml_str: &str) -> Result<()> {
        let toml_value: Value = toml::from_str(toml_str)?;

        if let Some(rpc) = toml_value.get("rpc") {
            if let Some(enable) = rpc.get("enable").and_then(|v| v.as_bool()) {
                self.rpc_enable = enable;
            }
            //self.rpc.deserialize_toml(rpc)?;
        }

        if let Some(node) = toml_value.get("node") {
            //self.node.deserialize_toml(node)?;
        }

        if let Some(opencl) = toml_value.get("opencl") {
            if let Some(enable) = opencl.get("enable").and_then(|v| v.as_bool()) {
                self.opencl_enable = enable;
            }
            if let Some(opencl_table) = opencl.as_table() {
                self.opencl.deserialize_toml(opencl_table)?;
            }
        }

        Ok(())
    }
}
