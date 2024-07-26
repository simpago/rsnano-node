use super::{FrontiersConfirmationMode, NodeConfig, NodeRpcConfig, OpenclConfig, Peer};
use crate::NetworkParams;
use anyhow::Result;
use rsnano_core::{utils::TomlWriter, Account, Amount};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

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
            node: NodeConfig::default(None, network_params, parallelism),
            opencl: OpenclConfig::new(),
            opencl_enable: false,
            rpc: NodeRpcConfig::default()?,
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
}

pub struct TomlDaemonConfig {
    pub(crate) rpc_enable: bool,
    pub(crate) rpc: TomlNodeRpcConfig,
    pub(crate) node: TomlNodeConfig,
    pub(crate) opencl: TomlOpenclConfig,
    pub(crate) opencl_enable: bool,
}

#[derive(Deserialize, Serialize)]
pub struct TomlNodeConfig {
    pub(crate) allow_local_peers: Option<bool>,
    pub(crate) background_threads: Option<u32>,
    pub(crate) backlog_scan_batch_size: Option<u32>,
    pub(crate) backlog_scan_frequency: Option<u32>,
    pub(crate) backup_before_upgrade: Option<bool>,
    pub(crate) bandwidth_limit: Option<usize>,
    pub(crate) bandwidth_limit_burst_ratio: Option<f64>,
    pub(crate) block_processor_batch_max_time_ms: Option<i64>,
    pub(crate) bootstrap_bandwidth_burst_ratio: Option<f64>,
    pub(crate) bootstrap_bandwidth_limit: Option<usize>,
    pub(crate) bootstrap_connections: Option<u32>,
    pub(crate) bootstrap_connections_max: Option<u32>,
    pub(crate) bootstrap_fraction_numerator: Option<u32>,
    pub(crate) bootstrap_frontier_request_count: Option<u32>,
    pub(crate) bootstrap_initiator_threads: Option<u32>,
    pub(crate) bootstrap_serving_threads: Option<u32>,
    pub(crate) confirming_set_batch_time: Option<Duration>,
    pub(crate) enable_voting: Option<bool>,
    pub(crate) external_address: Option<String>,
    pub(crate) external_port: Option<u16>,
    pub(crate) frontiers_confirmation: Option<FrontiersConfirmationMode>,
    pub(crate) io_threads: Option<u32>,
    pub(crate) max_queued_requests: Option<u32>,
    pub(crate) max_unchecked_blocks: Option<u32>,
    pub(crate) max_work_generate_multiplier: Option<f64>,
    pub(crate) network_threads: Option<u32>,
    pub(crate) online_weight_minimum: Option<Amount>,
    pub(crate) password_fanout: Option<u32>,
    pub(crate) peering_port: Option<u16>,
    pub(crate) pow_sleep_interval_ns: Option<i64>,
    pub(crate) preconfigured_peers: Option<Vec<String>>,
    pub(crate) preconfigured_representatives: Option<Vec<Account>>,
    pub(crate) receive_minimum: Option<Amount>,
    pub(crate) rep_crawler_weight_minimum: Option<Amount>,
    pub(crate) representative_vote_weight_minimum: Option<Amount>,
    pub(crate) request_aggregator_threads: Option<u32>,
    pub(crate) signature_checker_threads: Option<u32>,
    pub(crate) tcp_incoming_connections_max: Option<u32>,
    pub(crate) tcp_io_timeout_s: Option<i64>,
    pub(crate) unchecked_cutoff_time_s: Option<i64>,
    pub(crate) use_memory_pools: Option<bool>,
    pub(crate) vote_generator_delay_ms: Option<i64>,
    pub(crate) vote_generator_threshold: Option<u32>,
    pub(crate) vote_minimum: Option<Amount>,
    pub(crate) work_peers: Option<Vec<Peer>>,
    pub(crate) work_threads: Option<u32>,
}

impl TomlNodeConfig {
    pub fn default(
        peering_port: Option<u16>,
        network_params: &NetworkParams,
        parallelism: usize,
    ) -> Self {
        let default_config = NodeConfig::default(peering_port, network_params, parallelism);

        Self {
            allow_local_peers: Some(default_config.allow_local_peers),
            background_threads: Some(default_config.background_threads),
            backlog_scan_batch_size: Some(default_config.backlog_scan_batch_size),
            backlog_scan_frequency: Some(default_config.backlog_scan_frequency),
            backup_before_upgrade: Some(default_config.backup_before_upgrade),
            bandwidth_limit: Some(default_config.bandwidth_limit),
            bandwidth_limit_burst_ratio: Some(default_config.bandwidth_limit_burst_ratio),
            block_processor_batch_max_time_ms: Some(
                default_config.block_processor_batch_max_time_ms,
            ),
            bootstrap_bandwidth_burst_ratio: Some(default_config.bootstrap_bandwidth_burst_ratio),
            bootstrap_bandwidth_limit: Some(default_config.bootstrap_bandwidth_limit),
            bootstrap_connections: Some(default_config.bootstrap_connections),
            bootstrap_connections_max: Some(default_config.bootstrap_connections_max),
            bootstrap_fraction_numerator: Some(default_config.bootstrap_fraction_numerator),
            bootstrap_frontier_request_count: Some(default_config.bootstrap_frontier_request_count),
            bootstrap_initiator_threads: Some(default_config.bootstrap_initiator_threads),
            bootstrap_serving_threads: Some(default_config.bootstrap_serving_threads),
            confirming_set_batch_time: Some(default_config.confirming_set_batch_time),
            enable_voting: Some(default_config.enable_voting),
            external_address: Some(default_config.external_address.clone()),
            external_port: Some(default_config.external_port),
            frontiers_confirmation: Some(default_config.frontiers_confirmation),
            io_threads: Some(default_config.io_threads),
            max_queued_requests: Some(default_config.max_queued_requests),
            max_unchecked_blocks: Some(default_config.max_unchecked_blocks),
            max_work_generate_multiplier: Some(default_config.max_work_generate_multiplier),
            network_threads: Some(default_config.network_threads),
            online_weight_minimum: Some(default_config.online_weight_minimum),
            password_fanout: Some(default_config.password_fanout),
            peering_port: default_config.peering_port,
            pow_sleep_interval_ns: Some(default_config.pow_sleep_interval_ns),
            preconfigured_peers: Some(default_config.preconfigured_peers.clone()),
            preconfigured_representatives: Some(
                default_config.preconfigured_representatives.clone(),
            ),
            receive_minimum: Some(default_config.receive_minimum),
            rep_crawler_weight_minimum: Some(default_config.rep_crawler_weight_minimum),
            representative_vote_weight_minimum: Some(
                default_config.representative_vote_weight_minimum,
            ),
            request_aggregator_threads: Some(default_config.request_aggregator_threads),
            signature_checker_threads: Some(default_config.signature_checker_threads),
            tcp_incoming_connections_max: Some(default_config.tcp_incoming_connections_max),
            tcp_io_timeout_s: Some(default_config.tcp_io_timeout_s),
            unchecked_cutoff_time_s: Some(default_config.unchecked_cutoff_time_s),
            use_memory_pools: Some(default_config.use_memory_pools),
            vote_generator_delay_ms: Some(default_config.vote_generator_delay_ms),
            vote_generator_threshold: Some(default_config.vote_generator_threshold),
            vote_minimum: Some(default_config.vote_minimum),
            work_peers: Some(default_config.work_peers.clone()),
            work_threads: Some(default_config.work_threads),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct TomlNodeRpcConfig {
    pub enable_sign_hash: bool,
    pub child_process: TomlRpcChildProcessConfig,
}

impl TomlNodeRpcConfig {
    pub fn default() -> Result<Self> {
        let default_config = NodeRpcConfig::default()?;

        Ok(Self {
            enable_sign_hash: default_config.enable_sign_hash,
            child_process: TomlRpcChildProcessConfig {
                enable: default_config.child_process.enable,
                rpc_path: default_config.child_process.rpc_path,
            },
        })
    }
}

#[derive(Deserialize, Serialize)]
pub struct TomlRpcChildProcessConfig {
    pub enable: bool,
    pub rpc_path: PathBuf,
}

#[derive(Deserialize, Serialize)]
pub struct TomlOpenclConfig {
    pub platform: u32,
    pub device: u32,
    pub threads: u32,
}
