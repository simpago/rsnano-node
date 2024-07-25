use std::time::Duration;

use super::{FrontiersConfirmationMode, NodeConfig, NodeRpcConfig, OpenclConfig, Peer};
use crate::NetworkParams;
use anyhow::Result;
use rsnano_core::{
    utils::{get_cpu_count, TomlWriter},
    Account, Amount,
};
use serde::Deserialize;
use toml::Value;

#[derive(Deserialize)]
pub struct TomlNodeConfig {
    pub allow_local_peers: bool,
    pub background_threads: u32,
    pub backlog_scan_batch_size: u32,
    pub backlog_scan_frequency: u32,
    pub backup_before_upgrade: bool,
    pub bandwidth_limit: usize,
    pub bandwidth_limit_burst_ratio: f64,
    pub block_processor_batch_max_time_ms: i64,
    pub bootstrap_bandwidth_burst_ratio: f64,
    pub bootstrap_bandwidth_limit: usize,
    pub bootstrap_connections: u32,
    pub bootstrap_connections_max: u32,
    pub bootstrap_fraction_numerator: u32,
    pub bootstrap_frontier_request_count: u32,
    pub bootstrap_initiator_threads: u32,
    pub bootstrap_serving_threads: u32,
    pub confirming_set_batch_time: Duration,
    pub enable_voting: bool,
    pub external_address: String,
    pub external_port: u16,
    pub frontiers_confirmation: FrontiersConfirmationMode,
    pub io_threads: u32,
    pub max_queued_requests: u32,
    pub max_unchecked_blocks: u32,
    pub max_work_generate_multiplier: f64,
    pub network_threads: u32,
    pub online_weight_minimum: Amount,
    pub password_fanout: u32,
    pub peering_port: Option<u16>,
    pub pow_sleep_interval_ns: i64,
    pub preconfigured_peers: Vec<String>,
    pub preconfigured_representatives: Vec<Account>,
    pub receive_minimum: Amount,
    pub rep_crawler_weight_minimum: Amount,
    pub representative_vote_weight_minimum: Amount,
    pub request_aggregator_threads: u32,
    pub signature_checker_threads: u32,
    pub tcp_incoming_connections_max: u32,
    pub tcp_io_timeout_s: i64,
    pub unchecked_cutoff_time_s: i64,
    pub use_memory_pools: bool,
    pub vote_generator_delay_ms: i64,
    pub vote_generator_threshold: u32,
    pub vote_minimum: Amount,
    pub work_peers: Vec<Peer>,
    pub work_threads: u32,
}

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
            node: NodeConfig::new(
                Some(network_params.network.default_node_port),
                &network_params,
                get_cpu_count(),
            ),
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

        //toml.put_child("node", &mut |node| self.node.serialize_toml(node))?;

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
