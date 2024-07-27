use super::{NodeConfig, NodeRpcConfig, OpenclConfig};
use super::{NodeConfigToml, NodeRpcConfigToml, OpenclConfigToml};
use crate::NetworkParams;
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

pub struct DaemonConfig {
    pub rpc: NodeRpcConfig,
    pub node: NodeConfig,
    pub opencl: OpenclConfig,
}

impl DaemonConfig {
    pub fn new(network_params: &NetworkParams, parallelism: usize) -> Result<Self> {
        Ok(Self {
            node: NodeConfig::default(None, network_params, parallelism),
            opencl: OpenclConfig::new(),
            rpc: NodeRpcConfig::default()?,
        })
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_child("rpc", &mut |rpc| {
            self.rpc.serialize_toml(rpc)?;
            rpc.put_bool(
                "enable",
                self.rpc.enable,
                "Enable or disable RPC\ntype:bool",
            )?;
            Ok(())
        })?;

        toml.put_child("node", &mut |node| self.node.serialize_toml(node))?;

        toml.put_child("opencl", &mut |opencl| {
            self.opencl.serialize_toml(opencl)?;
            opencl.put_bool(
                "enable",
                self.opencl.enable,
                "Enable or disable OpenCL work generation\nIf enabled, consider freeing up CPU resources by setting [work_threads] to zero\ntype:bool",
            )?;
            Ok(())
        })?;

        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
pub struct DaemonConfigToml {
    pub node: Option<NodeConfigToml>,
    pub(crate) rpc: Option<NodeRpcConfigToml>,
    pub(crate) opencl: Option<OpenclConfigToml>,
}

impl DaemonConfigToml {
    pub fn merge_defaults(&self, default_config: &DaemonConfigToml) -> Result<String> {
        let defaults_str = toml::to_string(default_config)?;
        let current_str = toml::to_string(self)?;

        let mut result = String::new();
        let mut stream_defaults = defaults_str.lines();
        let mut stream_current = current_str.lines();

        while let (Some(line_defaults), Some(line_current)) =
            (stream_defaults.next(), stream_current.next())
        {
            if line_defaults == line_current {
                result.push_str(line_defaults);
                result.push('\n');
            } else {
                if let Some(pos) = line_current.find('#') {
                    result.push_str(&line_current[0..pos]);
                    result.push_str(&line_current[pos + 1..]);
                    result.push('\n');
                } else {
                    result.push_str(line_current);
                    result.push('\n');
                }
            }
        }

        Ok(result)
    }
}

impl From<DaemonConfig> for DaemonConfigToml {
    fn from(config: DaemonConfig) -> Self {
        Self {
            node: Some(config.node.into()),
            rpc: Some(config.rpc.into()),
            opencl: Some(config.opencl.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{DaemonConfig, DaemonConfigToml, NetworkConstants},
        NetworkParams,
    };

    #[test]
    fn test_toml_serialization() {
        let network_params = NetworkParams::new(NetworkConstants::active_network());
        let config: DaemonConfigToml = DaemonConfig::new(&network_params, 0).unwrap().into();
        let toml_str = toml::to_string(&config).unwrap();

        let deserialized_config: DaemonConfigToml = toml::from_str(&toml_str).unwrap();

        assert_eq!(
            serde_json::to_string(&config).unwrap(),
            serde_json::to_string(&deserialized_config).unwrap()
        );
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_str = r#"
            [node]
           	allow_local_peers = true
           	background_threads = 4
           	backlog_scan_batch_size = 10000
           	backlog_scan_frequency = 10
           	backup_before_upgrade = false
           	bandwidth_limit = 10485760
           	bandwidth_limit_burst_ratio = 3.0
           	block_processor_batch_max_time_ms = 500
           	bootstrap_bandwidth_burst_ratio = 1.0
           	bootstrap_bandwidth_limit = 5242880
           	bootstrap_connections = 4
           	bootstrap_connections_max = 64
           	bootstrap_fraction_numerator = 1
           	bootstrap_frontier_request_count = 1048576
           	bootstrap_initiator_threads = 1
           	bootstrap_serving_threads = 1
           	confirming_set_batch_time = "250"
           	enable_voting = false
           	external_address = "::"
           	external_port = 0
           	frontiers_confirmation = "Automatic"
           	io_threads = 4
           	max_queued_requests = 512
           	max_unchecked_blocks = 65536
           	max_work_generate_multiplier = 64.0
           	network_threads = 4
           	online_weight_minimum = "60000000000000000000000000000000000000"
           	password_fanout = 1024
           	peering_port = 54000
           	pow_sleep_interval_ns = 0
           	preconfigured_peers = ["peering-beta.nano.org"]
           	preconfigured_representatives = ["nano_1defau1t9off1ine9rep99999999999999999999999999999999wgmuzxxy"]
           	receive_minimum = "1000000000000000000000000"
           	rep_crawler_weight_minimum = "340282366920938463463374607431768211455"
           	representative_vote_weight_minimum = "10000000000000000000000000000000"
           	request_aggregator_threads = 4
           	signature_checker_threads = 0
           	tcp_incoming_connections_max = 2048
           	tcp_io_timeout_s = 15
           	unchecked_cutoff_time_s = 14400
           	use_memory_pools = true
           	vote_generator_delay_ms = 100
           	vote_generator_threshold = 3
           	vote_minimum = "1000000000000000000000000000000000"
           	work_peers = []
           	work_threads = 4
           	max_pruning_age_s = 300
           	max_pruning_depth = 0
           	callback_address = ""
           	callback_port = 0
           	callback_target = ""
           	rep_crawler_query_timeout = "60000"
           	priority_scheduler_enabled = true

            [node.optimistic_scheduler]
           	enabled = true
           	gap_threshold = 32
           	max_size = 65536

            [node.priority_bucket]
           	max_blocks = 8192
           	reserved_elections = 100
           	max_elections = 150

            [node.bootstrap_ascending]
           	requests_limit = 64
           	database_requests_limit = 1024
           	pull_count = 128
           	throttle_coefficient = 16
           	block_wait_count = 1000

            [node.bootstrap_ascending.timeout]
           	secs = 3
           	nanos = 0

            [node.bootstrap_ascending.throttle_wait]
           	secs = 0
           	nanos = 100000000

            [node.bootstrap_ascending.account_sets]
           	consideration_count = 4
           	priorities_max = 262144
           	blocking_max = 262144

            [node.bootstrap_ascending.account_sets.cooldown]
           	secs = 3
           	nanos = 0

            [node.bootstrap_server]
           	max_queue = 16
           	threads = 1
           	batch_size = 64

            [[node.secondary_work_peers]]
           	address = "127.0.0.1"
           	port = 8076

            [node.websocket_config]
           	enabled = false
           	port = 57000
           	address = "::1"

            [node.ipc_config.transport_domain]
           	path = "/tmp/nano"

            [node.ipc_config.transport_domain.transport]
           	enabled = false
           	allow_unsafe = false
           	io_timeout = 15
           	io_threads = -1

            [node.ipc_config.transport_tcp]
           	port = 56000

            [node.ipc_config.transport_tcp.transport]
           	enabled = false
           	allow_unsafe = false
           	io_timeout = 15
           	io_threads = -1

            [node.ipc_config.flatbuffers]
           	skip_unexpected_fields_in_json = true
           	verify_buffers = true

            [node.diagnostics_config.txn_tracking]
           	enable = false
           	min_read_txn_time_ms = 5000
           	min_write_txn_time_ms = 500
           	ignore_writes_below_block_processor_max_time = true

            [node.stat_config]
           	max_samples = 16384
           	log_rotation_count = 100
           	log_headers = true
           	log_counters_filename = "counters.stat"
           	log_samples_filename = "samples.stat"

            [node.stat_config.log_samples_interval]
           	secs = 0
           	nanos = 0

            [node.stat_config.log_counters_interval]
           	secs = 0
           	nanos = 0

            [node.lmdb_config]
           	sync = "Always"
           	max_databases = 128
           	map_size = 274877906944

            [node.vote_cache]
           	max_size = 65536
           	max_voters = 64

            [node.vote_cache.age_cutoff]
           	secs = 900
           	nanos = 0

            [node.block_processor]
           	max_peer_queue = 128
           	max_system_queue = 16384
           	priority_live = 1
           	priority_bootstrap = 8
           	priority_local = 16

            [node.active_elections]
           	size = 5000
           	hinted_limit_percentage = 20
           	optimistic_limit_percentage = 10
           	confirmation_history_size = 2048
           	confirmation_cache = 65536

            [node.vote_processor]
           	max_pr_queue = 256
           	max_non_pr_queue = 32
           	pr_priority = 3
           	threads = 1
           	batch_size = 1024
           	max_triggered = 16384

            [node.tcp]
           	max_inbound_connections = 2048
           	max_outbound_connections = 2048
           	max_attempts = 60
           	max_attempts_per_ip = 1

            [node.tcp.connect_timeout]
           	secs = 60
           	nanos = 0

            [node.request_aggregator]
           	threads = 1
           	max_queue = 128
           	batch_size = 16

            [node.message_processor]
           	threads = 1
           	max_queue = 64

            [node.local_block_broadcaster]
           	max_size = 8192
           	broadcast_rate_limit = 32
           	broadcast_rate_burst_ratio = 3.0

            [node.local_block_broadcaster.rebroadcast_interval]
           	secs = 3
           	nanos = 0

            [node.local_block_broadcaster.max_rebroadcast_interval]
           	secs = 60
           	nanos = 0

            [node.local_block_broadcaster.cleanup_interval]
           	secs = 60
           	nanos = 0

            [node.confirming_set]
           	max_blocks = 8192
           	max_queued_notifications = 8

            [node.monitor]
           	enabled = true

            [node.monitor.interval]
           	secs = 60
           	nanos = 0

            [rpc]
            enable = false
           	enable_sign_hash = false

            [rpc.child_process]
           	enable = false
           	rpc_path = "/Users/ruimorais/rsnano/rust/../build/cargo/debug/nano_rpc"

            [opencl]
           	platform = 0
           	device = 0
           	threads = 1048576
        "#;

        toml::from_str::<DaemonConfigToml>(toml_str).unwrap();
    }
}
