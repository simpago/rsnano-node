use super::{
    BlockProcessorToml, BootstrapAscendingToml, DiagnosticsConfig, FrontiersConfirmationMode,
    HintedSchedulerConfig, MonitorConfig, NodeConfig, NodeRpcConfig, OpenclConfig,
    OptimisticSchedulerConfig, Peer, WebsocketConfig,
};
use crate::{
    block_processing::LocalBlockBroadcasterConfig,
    bootstrap::BootstrapServerConfig,
    cementation::ConfirmingSetConfig,
    consensus::{
        ActiveElectionsConfig, PriorityBucketConfig, RequestAggregatorConfig, VoteCacheConfig,
        VoteProcessorConfig,
    },
    stats::StatsConfig,
    transport::{MessageProcessorConfig, TcpConfig},
    IpcConfig, NetworkParams,
};
use anyhow::Result;
use rsnano_core::{utils::TomlWriter, Account, Amount};
use rsnano_store_lmdb::LmdbConfig;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

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
                self.rpc.rpc_enable,
                "Enable or disable RPC\ntype:bool",
            )?;
            Ok(())
        })?;

        toml.put_child("node", &mut |node| self.node.serialize_toml(node))?;

        toml.put_child("opencl", &mut |opencl| {
            self.opencl.serialize_toml(opencl)?;
            opencl.put_bool(
                "enable",
                self.opencl.opencl_enable,
                "Enable or disable OpenCL work generation\nIf enabled, consider freeing up CPU resources by setting [work_threads] to zero\ntype:bool",
            )?;
            Ok(())
        })?;

        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
pub struct TomlDaemonConfig {
    pub node: Option<TomlNodeConfig>,
    pub(crate) rpc: Option<TomlNodeRpcConfig>,
    pub(crate) opencl: Option<TomlOpenclConfig>,
}

impl TomlDaemonConfig {
    pub fn default(network_params: &NetworkParams, parallelism: usize) -> Result<Self> {
        Ok(Self {
            rpc: Some(TomlNodeRpcConfig::default()?),
            node: Some(TomlNodeConfig::default(&network_params, parallelism)),
            opencl: Some(TomlOpenclConfig::default()),
        })
    }

    pub fn merge_defaults(&self, default_config: &TomlDaemonConfig) -> Result<String> {
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
    pub(crate) confirming_set_batch_time: Option<Miliseconds>,
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
    pub(crate) optimistic_scheduler: Option<OptimisticSchedulerConfig>,
    pub(crate) hinted_scheduler: Option<HintedSchedulerConfig>,
    pub(crate) priority_bucket: Option<PriorityBucketConfig>,
    pub(crate) bootstrap_ascending: Option<BootstrapAscendingToml>,
    pub(crate) bootstrap_server: Option<BootstrapServerConfig>,
    pub(crate) secondary_work_peers: Option<Vec<Peer>>,
    pub(crate) max_pruning_age_s: Option<i64>,
    pub(crate) max_pruning_depth: Option<u64>,
    pub(crate) callback_address: Option<String>,
    pub(crate) callback_port: Option<u16>,
    pub(crate) callback_target: Option<String>,
    pub(crate) websocket_config: Option<WebsocketConfig>,
    pub(crate) ipc_config: Option<IpcConfig>,
    pub(crate) diagnostics_config: Option<DiagnosticsConfig>,
    pub(crate) stat_config: Option<StatsConfig>,
    pub(crate) lmdb_config: Option<LmdbConfig>,
    pub(crate) vote_cache: Option<VoteCacheConfig>,
    pub(crate) rep_crawler_query_timeout: Option<Miliseconds>,
    pub(crate) block_processor: Option<BlockProcessorToml>,
    pub(crate) active_elections: Option<ActiveElectionsConfig>,
    pub(crate) vote_processor: Option<VoteProcessorConfig>,
    pub(crate) tcp: Option<TcpConfig>,
    pub(crate) request_aggregator: Option<RequestAggregatorConfig>,
    pub(crate) message_processor: Option<MessageProcessorConfig>,
    pub(crate) priority_scheduler_enabled: Option<bool>,
    pub(crate) local_block_broadcaster: Option<LocalBlockBroadcasterConfig>,
    pub(crate) confirming_set: Option<ConfirmingSetConfig>,
    pub(crate) monitor: Option<MonitorConfig>,
}

impl TomlNodeConfig {
    pub fn default(network_params: &NetworkParams, parallelism: usize) -> Self {
        let default_config = NodeConfig::default(
            Some(network_params.network.default_node_port),
            network_params,
            parallelism,
        );

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
            confirming_set_batch_time: Some(Miliseconds(
                default_config.confirming_set_batch_time.as_millis(),
            )),
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
            work_peers: Some(default_config.work_peers),
            work_threads: Some(default_config.work_threads),
            optimistic_scheduler: Some(default_config.optimistic_scheduler),
            hinted_scheduler: Some(default_config.hinted_scheduler),
            priority_bucket: Some(default_config.priority_bucket),
            bootstrap_ascending: Some(default_config.bootstrap_ascending),
            bootstrap_server: Some(default_config.bootstrap_server),
            secondary_work_peers: Some(default_config.secondary_work_peers),
            max_pruning_age_s: Some(default_config.max_pruning_age_s),
            max_pruning_depth: Some(default_config.max_pruning_depth),
            callback_address: Some(default_config.callback_address),
            callback_port: Some(default_config.callback_port),
            callback_target: Some(default_config.callback_target),
            websocket_config: Some(default_config.websocket_config),
            ipc_config: Some(default_config.ipc_config),
            diagnostics_config: Some(default_config.diagnostics_config),
            stat_config: Some(default_config.stat_config),
            lmdb_config: Some(default_config.lmdb_config),
            vote_cache: Some(default_config.vote_cache),
            rep_crawler_query_timeout: Some(Miliseconds(
                default_config.rep_crawler_query_timeout.as_millis(),
            )),
            block_processor: Some(default_config.block_processor),
            active_elections: Some(default_config.active_elections),
            vote_processor: Some(default_config.vote_processor),
            tcp: Some(default_config.tcp),
            request_aggregator: Some(default_config.request_aggregator),
            message_processor: Some(default_config.message_processor),
            priority_scheduler_enabled: Some(default_config.priority_scheduler_enabled),
            local_block_broadcaster: Some(default_config.local_block_broadcaster),
            confirming_set: Some(default_config.confirming_set),
            monitor: Some(default_config.monitor),
        }
    }
}

pub(crate) struct Miliseconds(pub(crate) u128);

impl Serialize for Miliseconds {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Miliseconds {
    fn deserialize<D>(deserializer: D) -> Result<Miliseconds, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let miliseconds = s.parse::<u128>().map_err(de::Error::custom)?;
        Ok(Miliseconds(miliseconds))
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
impl TomlOpenclConfig {
    fn default() -> TomlOpenclConfig {
        let default_config = OpenclConfig::default();

        Self {
            platform: default_config.platform,
            device: default_config.device,
            threads: default_config.threads,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{NetworkConstants, TomlDaemonConfig},
        NetworkParams,
    };

    #[test]
    fn test_toml_serialization() {
        let network_params = NetworkParams::new(NetworkConstants::active_network());
        let config = TomlDaemonConfig::default(&network_params, 0).unwrap();
        let toml_str = toml::to_string(&config).unwrap();

        let deserialized_config: TomlDaemonConfig = toml::from_str(&toml_str).unwrap();

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

            [node.hinted_scheduler]
           	enabled = true
           	hinting_theshold_percent = 10
           	vacancy_threshold_percent = 20

            [node.hinted_scheduler.check_interval]
           	secs = 1
           	nanos = 0

            [node.hinted_scheduler.block_cooldown]
           	secs = 5
           	nanos = 0

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
           	enable_sign_hash = false

            [rpc.child_process]
           	enable = false
           	rpc_path = "/Users/ruimorais/rsnano/rust/../build/cargo/debug/nano_rpc"

            [opencl]
           	platform = 0
           	device = 0
           	threads = 1048576
        "#;

        toml::from_str::<TomlDaemonConfig>(toml_str).unwrap();
    }
}
