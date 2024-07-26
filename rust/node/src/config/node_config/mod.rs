use crate::{
    block_processing::LocalBlockBroadcasterConfig,
    bootstrap::BootstrapInitiatorConfig,
    cementation::ConfirmingSetConfig,
    consensus::{
        ActiveElectionsConfig, PriorityBucketConfig, RequestAggregatorConfig, VoteCacheConfig,
        VoteProcessorConfig,
    },
    stats::StatsConfig,
    toml::NodeConfigToml,
    transport::{MessageProcessorConfig, TcpConfig},
    IpcConfig, NetworkParams, DEV_NETWORK_PARAMS,
};
use anyhow::Result;
use once_cell::sync::Lazy;
use rand::{thread_rng, Rng};
use rsnano_core::{
    utils::{get_env_or_default_string, is_sanitizer_build, TomlWriter},
    Account, Amount, Networks, GXRB_RATIO, XRB_RATIO,
};
use rsnano_store_lmdb::LmdbConfig;
use serde::{Deserialize, Serialize};
use std::{cmp::max, net::Ipv6Addr, time::Duration};

mod block_processor;
mod bootstrap_ascending;
mod bootstrap_server;
mod diagnostics_config;
mod node_rpc_config;
mod opencl_config;
mod optimistic_scheduler_config;
mod websocket_config;

pub use block_processor::*;
pub use bootstrap_ascending::*;
pub use bootstrap_server::*;
pub use diagnostics_config::*;
pub use node_rpc_config::*;
pub use opencl_config::*;
pub use optimistic_scheduler_config::*;
pub use websocket_config::*;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, FromPrimitive, Deserialize, Serialize)]
pub enum FrontiersConfirmationMode {
    Always,    // Always confirm frontiers
    Automatic, // Always mode if node contains representative with at least 50% of principal weight, less frequest requests if not
    Disabled,  // Do not confirm frontiers
    Invalid,
}

#[derive(Clone)]
pub struct NodeConfig {
    pub peering_port: Option<u16>,
    pub optimistic_scheduler: OptimisticSchedulerConfig,
    pub hinted_scheduler: HintedSchedulerConfig,
    pub priority_bucket: PriorityBucketConfig,
    pub bootstrap_fraction_numerator: u32,
    pub receive_minimum: Amount,
    pub online_weight_minimum: Amount,
    /// The minimum vote weight that a representative must have for its vote to be counted.
    /// All representatives above this weight will be kept in memory!
    pub representative_vote_weight_minimum: Amount,
    pub password_fanout: u32,
    pub io_threads: u32,
    pub network_threads: u32,
    pub work_threads: u32,
    pub background_threads: u32,
    pub signature_checker_threads: u32,
    pub enable_voting: bool,
    pub bootstrap_connections: u32,
    pub bootstrap_connections_max: u32,
    pub bootstrap_initiator_threads: u32,
    pub bootstrap_serving_threads: u32,
    pub bootstrap_frontier_request_count: u32,
    pub block_processor_batch_max_time_ms: i64,
    pub allow_local_peers: bool,
    pub vote_minimum: Amount,
    pub vote_generator_delay_ms: i64,
    pub vote_generator_threshold: u32,
    pub unchecked_cutoff_time_s: i64,
    pub tcp_io_timeout_s: i64,
    pub pow_sleep_interval_ns: i64,
    pub external_address: String,
    pub external_port: u16,
    pub tcp_incoming_connections_max: u32,
    pub use_memory_pools: bool,
    pub bandwidth_limit: usize,
    pub bandwidth_limit_burst_ratio: f64,
    pub bootstrap_ascending: BootstrapAscendingConfig,
    pub bootstrap_server: BootstrapServerConfig,
    pub bootstrap_bandwidth_limit: usize,
    pub bootstrap_bandwidth_burst_ratio: f64,
    pub confirming_set_batch_time: Duration,
    pub backup_before_upgrade: bool,
    pub max_work_generate_multiplier: f64,
    pub frontiers_confirmation: FrontiersConfirmationMode,
    pub max_queued_requests: u32,
    pub request_aggregator_threads: u32,
    pub max_unchecked_blocks: u32,
    pub rep_crawler_weight_minimum: Amount,
    pub work_peers: Vec<Peer>,
    pub secondary_work_peers: Vec<Peer>,
    pub preconfigured_peers: Vec<String>,
    pub preconfigured_representatives: Vec<Account>,
    pub max_pruning_age_s: i64,
    pub max_pruning_depth: u64,
    pub callback_address: String,
    pub callback_port: u16,
    pub callback_target: String,
    pub websocket_config: WebsocketConfig,
    pub ipc_config: IpcConfig,
    pub diagnostics_config: DiagnosticsConfig,
    pub stat_config: StatsConfig,
    pub lmdb_config: LmdbConfig,
    /// Number of accounts per second to process when doing backlog population scan
    pub backlog_scan_batch_size: u32,
    /// Number of times per second to run backlog population batches. Number of accounts per single batch is `backlog_scan_batch_size / backlog_scan_frequency`
    pub backlog_scan_frequency: u32,
    pub vote_cache: VoteCacheConfig,
    pub rep_crawler_query_timeout: Duration,
    pub block_processor: BlockProcessorConfig,
    pub active_elections: ActiveElectionsConfig,
    pub vote_processor: VoteProcessorConfig,
    pub tcp: TcpConfig,
    pub request_aggregator: RequestAggregatorConfig,
    pub message_processor: MessageProcessorConfig,
    pub priority_scheduler_enabled: bool,
    pub local_block_broadcaster: LocalBlockBroadcasterConfig,
    pub confirming_set: ConfirmingSetConfig,
    pub monitor: MonitorConfig,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Peer {
    pub address: String,
    pub port: u16,
}

impl Peer {
    pub fn new(address: impl Into<String>, port: u16) -> Self {
        Self {
            address: address.into(),
            port,
        }
    }
}

static DEFAULT_LIVE_PEER_NETWORK: Lazy<String> =
    Lazy::new(|| get_env_or_default_string("NANO_DEFAULT_PEER", "peering.nano.org"));

static DEFAULT_BETA_PEER_NETWORK: Lazy<String> =
    Lazy::new(|| get_env_or_default_string("NANO_DEFAULT_PEER", "peering-beta.nano.org"));

static DEFAULT_TEST_PEER_NETWORK: Lazy<String> =
    Lazy::new(|| get_env_or_default_string("NANO_DEFAULT_PEER", "peering-test.nano.org"));

impl NodeConfig {
    pub fn default(
        peering_port: Option<u16>,
        network_params: &NetworkParams,
        parallelism: usize,
    ) -> Self {
        if peering_port == Some(0) {
            // comment for posterity:
            // - we used to consider ports being 0 a sentinel that meant to use a default port for that specific purpose
            // - the actual default value was determined based on the active network (e.g. dev network peering port = 44000)
            // - now, the 0 value means something different instead: user wants to let the OS pick a random port
            // - for the specific case of the peering port, after it gets picked, it can be retrieved by client code via
            //   node.network.endpoint ().port ()
            // - the config value does not get back-propagated because it represents the choice of the user, and that was 0
        }

        let mut enable_voting = false;
        let mut preconfigured_peers = Vec::new();
        let mut preconfigured_representatives = Vec::new();
        match network_params.network.current_network {
            Networks::NanoDevNetwork => {
                enable_voting = true;
                preconfigured_representatives.push(network_params.ledger.genesis_account);
            }
            Networks::NanoBetaNetwork => {
                preconfigured_peers.push(DEFAULT_BETA_PEER_NETWORK.clone());
                preconfigured_representatives.push(
                    Account::decode_account(
                        "nano_1defau1t9off1ine9rep99999999999999999999999999999999wgmuzxxy",
                    )
                    .unwrap(),
                );
            }
            Networks::NanoLiveNetwork => {
                preconfigured_peers.push(DEFAULT_LIVE_PEER_NETWORK.clone());
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "A30E0A32ED41C8607AA9212843392E853FCBCB4E7CB194E35C94F07F91DE59EF",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "67556D31DDFC2A440BF6147501449B4CB9572278D034EE686A6BEE29851681DF",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "5C2FBB148E006A8E8BA7A75DD86C9FE00C83F5FFDBFD76EAA09531071436B6AF",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "AE7AC63990DAAAF2A69BF11C913B928844BF5012355456F2F164166464024B29",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "BD6267D6ECD8038327D2BCC0850BDF8F56EC0414912207E81BCF90DFAC8A4AAA",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "2399A083C600AA0572F5E36247D978FCFC840405F8D4B6D33161C0066A55F431",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "2298FAB7C61058E77EA554CB93EDEEDA0692CBFCC540AB213B2836B29029E23A",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "3FE80B4BC842E82C1C18ABFEEC47EA989E63953BC82AC411F304D13833D52A56",
                    )
                    .unwrap(),
                );
            }
            Networks::NanoTestNetwork => {
                preconfigured_peers.push(DEFAULT_TEST_PEER_NETWORK.clone());
                preconfigured_representatives.push(network_params.ledger.genesis_account);
            }
            Networks::Invalid => panic!("invalid network"),
        }

        Self {
            peering_port,
            bootstrap_fraction_numerator: 1,
            receive_minimum: Amount::raw(*XRB_RATIO),
            online_weight_minimum: Amount::nano(60_000_000),
            representative_vote_weight_minimum: Amount::nano(10),
            password_fanout: 1024,
            io_threads: max(parallelism, 4) as u32,
            network_threads: max(parallelism, 4) as u32,
            work_threads: max(parallelism, 4) as u32,
            background_threads: max(parallelism, 4) as u32,
            /* Use half available threads on the system for signature checking. The calling thread does checks as well, so these are extra worker threads */
            signature_checker_threads: (parallelism / 2) as u32,
            enable_voting,
            bootstrap_connections: BootstrapInitiatorConfig::default().bootstrap_connections,
            bootstrap_connections_max: BootstrapInitiatorConfig::default()
                .bootstrap_connections_max,
            bootstrap_initiator_threads: 1,
            bootstrap_serving_threads: 1,
            bootstrap_frontier_request_count: BootstrapInitiatorConfig::default()
                .frontier_request_count,
            block_processor_batch_max_time_ms: BlockProcessorConfig::default()
                .batch_max_time
                .as_millis() as i64,
            allow_local_peers: !(network_params.network.is_live_network()
                || network_params.network.is_test_network()), // disable by default for live network
            vote_minimum: Amount::raw(*GXRB_RATIO),
            vote_generator_delay_ms: 100,
            vote_generator_threshold: 3,
            unchecked_cutoff_time_s: 4 * 60 * 60, // 4 hours
            tcp_io_timeout_s: if network_params.network.is_dev_network() && !is_sanitizer_build() {
                5
            } else {
                15
            },
            pow_sleep_interval_ns: 0,
            external_address: Ipv6Addr::UNSPECIFIED.to_string(),
            external_port: 0,
            // Default maximum incoming TCP connections, including realtime network & bootstrap
            tcp_incoming_connections_max: 2048,
            use_memory_pools: true,
            // Default outbound traffic shaping is 10MB/s
            bandwidth_limit: 10 * 1024 * 1024,
            // By default, allow bursts of 15MB/s (not sustainable)
            bandwidth_limit_burst_ratio: 3_f64,
            // Default boostrap outbound traffic limit is 5MB/s
            bootstrap_bandwidth_limit: 5 * 1024 * 1024,
            // Bootstrap traffic does not need bursts
            bootstrap_bandwidth_burst_ratio: 1.,
            bootstrap_ascending: Default::default(),
            bootstrap_server: Default::default(),
            confirming_set_batch_time: Duration::from_millis(250),
            backup_before_upgrade: false,
            max_work_generate_multiplier: 64_f64,
            frontiers_confirmation: FrontiersConfirmationMode::Automatic,
            max_queued_requests: 512,
            request_aggregator_threads: max(parallelism, 4) as u32,
            max_unchecked_blocks: 65536,
            rep_crawler_weight_minimum: Amount::decode_hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
                .unwrap(),
            work_peers: Vec::new(),
            secondary_work_peers: vec![Peer::new("127.0.0.1", 8076)],
            preconfigured_peers,
            preconfigured_representatives,
            max_pruning_age_s: if !network_params.network.is_beta_network() {
                24 * 60 * 60
            } else {
                5 * 60
            }, // 1 day; 5 minutes for beta network
            max_pruning_depth: 0,
            callback_address: String::new(),
            callback_port: 0,
            callback_target: String::new(),
            websocket_config: WebsocketConfig::new(&network_params.network),
            ipc_config: IpcConfig::new(&network_params.network),
            diagnostics_config: DiagnosticsConfig::new(),
            stat_config: StatsConfig::new(),
            lmdb_config: LmdbConfig::new(),
            backlog_scan_batch_size: 10 * 1000,
            backlog_scan_frequency: 10,
            optimistic_scheduler: OptimisticSchedulerConfig::new(),
            hinted_scheduler: if network_params.network.is_dev_network() {
                HintedSchedulerConfig::default_for_dev_network()
            } else {
                HintedSchedulerConfig::default()
            },
            priority_bucket: Default::default(),
            vote_cache: Default::default(),
            active_elections: Default::default(),
            rep_crawler_query_timeout: if network_params.network.is_dev_network() {
                Duration::from_secs(1)
            } else {
                Duration::from_secs(60)
            },
            block_processor: BlockProcessorConfig::new(),
            vote_processor: VoteProcessorConfig::new(parallelism),
            tcp: if network_params.network.is_dev_network() {
                TcpConfig::for_dev_network()
            } else {
                Default::default()
            },
            request_aggregator: RequestAggregatorConfig::new(parallelism),
            message_processor: MessageProcessorConfig::new(parallelism),
            priority_scheduler_enabled: true,
            local_block_broadcaster: LocalBlockBroadcasterConfig::new(
                network_params.network.current_network,
            ),
            confirming_set: Default::default(),
            monitor: Default::default(),
        }
    }

    pub fn config_override(&mut self, toml: &NodeConfigToml) {
        if let Some(allow_local_peers) = toml.allow_local_peers {
            self.allow_local_peers = allow_local_peers;
        }
        if let Some(background_threads) = toml.background_threads {
            self.background_threads = background_threads;
        }
        if let Some(backlog_scan_batch_size) = toml.backlog_scan_batch_size {
            self.backlog_scan_batch_size = backlog_scan_batch_size;
        }
        if let Some(backlog_scan_frequency) = toml.backlog_scan_frequency {
            self.backlog_scan_frequency = backlog_scan_frequency;
        }
        if let Some(backup_before_upgrade) = toml.backup_before_upgrade {
            self.backup_before_upgrade = backup_before_upgrade;
        }
        if let Some(bandwidth_limit) = toml.bandwidth_limit {
            self.bandwidth_limit = bandwidth_limit;
        }
        if let Some(bandwidth_limit_burst_ratio) = toml.bandwidth_limit_burst_ratio {
            self.bandwidth_limit_burst_ratio = bandwidth_limit_burst_ratio;
        }
        if let Some(block_processor_batch_max_time_ms) = toml.block_processor_batch_max_time_ms {
            self.block_processor_batch_max_time_ms = block_processor_batch_max_time_ms;
        }
        if let Some(bootstrap_bandwidth_burst_ratio) = toml.bootstrap_bandwidth_burst_ratio {
            self.bootstrap_bandwidth_burst_ratio = bootstrap_bandwidth_burst_ratio;
        }
        if let Some(bootstrap_bandwidth_limit) = toml.bootstrap_bandwidth_limit {
            self.bootstrap_bandwidth_limit = bootstrap_bandwidth_limit;
        }
        if let Some(bootstrap_connections) = toml.bootstrap_connections {
            self.bootstrap_connections = bootstrap_connections;
        }
        if let Some(bootstrap_connections_max) = toml.bootstrap_connections_max {
            self.bootstrap_connections_max = bootstrap_connections_max;
        }
        if let Some(bootstrap_fraction_numerator) = toml.bootstrap_fraction_numerator {
            self.bootstrap_fraction_numerator = bootstrap_fraction_numerator;
        }
        if let Some(bootstrap_frontier_request_count) = toml.bootstrap_frontier_request_count {
            self.bootstrap_frontier_request_count = bootstrap_frontier_request_count;
        }
        if let Some(bootstrap_initiator_threads) = toml.bootstrap_initiator_threads {
            self.bootstrap_initiator_threads = bootstrap_initiator_threads;
        }
        if let Some(bootstrap_serving_threads) = toml.bootstrap_serving_threads {
            self.bootstrap_serving_threads = bootstrap_serving_threads;
        }
        if let Some(confirming_set_batch_time) = &toml.confirming_set_batch_time {
            self.confirming_set_batch_time =
                Duration::from_millis(confirming_set_batch_time.0 as u64);
        }
        if let Some(enable_voting) = toml.enable_voting {
            self.enable_voting = enable_voting;
        }
        if let Some(external_address) = &toml.external_address {
            self.external_address = external_address.clone();
        }
        if let Some(external_port) = toml.external_port {
            self.external_port = external_port;
        }
        if let Some(frontiers_confirmation) = toml.frontiers_confirmation {
            self.frontiers_confirmation = frontiers_confirmation;
        }
        if let Some(io_threads) = toml.io_threads {
            self.io_threads = io_threads;
        }
        if let Some(max_queued_requests) = toml.max_queued_requests {
            self.max_queued_requests = max_queued_requests;
        }
        if let Some(max_unchecked_blocks) = toml.max_unchecked_blocks {
            self.max_unchecked_blocks = max_unchecked_blocks;
        }
        if let Some(max_work_generate_multiplier) = toml.max_work_generate_multiplier {
            self.max_work_generate_multiplier = max_work_generate_multiplier;
        }
        if let Some(network_threads) = toml.network_threads {
            self.network_threads = network_threads;
        }
        if let Some(online_weight_minimum) = toml.online_weight_minimum {
            self.online_weight_minimum = online_weight_minimum;
        }
        if let Some(password_fanout) = toml.password_fanout {
            self.password_fanout = password_fanout;
        }
        if let Some(peering_port) = toml.peering_port {
            self.peering_port = Some(peering_port);
        }
        if let Some(pow_sleep_interval_ns) = toml.pow_sleep_interval_ns {
            self.pow_sleep_interval_ns = pow_sleep_interval_ns;
        }
        if let Some(preconfigured_peers) = &toml.preconfigured_peers {
            self.preconfigured_peers = preconfigured_peers.clone();
        }
        if let Some(preconfigured_representatives) = &toml.preconfigured_representatives {
            self.preconfigured_representatives = preconfigured_representatives.clone();
        }
        if let Some(receive_minimum) = toml.receive_minimum {
            self.receive_minimum = receive_minimum;
        }
        if let Some(rep_crawler_weight_minimum) = toml.rep_crawler_weight_minimum {
            self.rep_crawler_weight_minimum = rep_crawler_weight_minimum;
        }
        if let Some(representative_vote_weight_minimum) = toml.representative_vote_weight_minimum {
            self.representative_vote_weight_minimum = representative_vote_weight_minimum;
        }
        if let Some(request_aggregator_threads) = toml.request_aggregator_threads {
            self.request_aggregator_threads = request_aggregator_threads;
        }
        if let Some(signature_checker_threads) = toml.signature_checker_threads {
            self.signature_checker_threads = signature_checker_threads;
        }
        if let Some(tcp_incoming_connections_max) = toml.tcp_incoming_connections_max {
            self.tcp_incoming_connections_max = tcp_incoming_connections_max;
        }
        if let Some(tcp_io_timeout_s) = toml.tcp_io_timeout_s {
            self.tcp_io_timeout_s = tcp_io_timeout_s;
        }
        if let Some(unchecked_cutoff_time_s) = toml.unchecked_cutoff_time_s {
            self.unchecked_cutoff_time_s = unchecked_cutoff_time_s;
        }
        if let Some(use_memory_pools) = toml.use_memory_pools {
            self.use_memory_pools = use_memory_pools;
        }
        if let Some(vote_generator_delay_ms) = toml.vote_generator_delay_ms {
            self.vote_generator_delay_ms = vote_generator_delay_ms;
        }
        if let Some(vote_generator_threshold) = toml.vote_generator_threshold {
            self.vote_generator_threshold = vote_generator_threshold;
        }
        if let Some(vote_minimum) = toml.vote_minimum {
            self.vote_minimum = vote_minimum;
        }
        if let Some(work_peers) = &toml.work_peers {
            self.work_peers = work_peers.clone();
        }
        if let Some(work_threads) = toml.work_threads {
            self.work_threads = work_threads;
        }

        if let Some(optimistic_scheduler) = &toml.optimistic_scheduler {
            self.optimistic_scheduler = optimistic_scheduler.clone();
        }
        if let Some(hinted_scheduler) = &toml.hinted_scheduler {
            self.hinted_scheduler = hinted_scheduler.clone();
        }
        if let Some(priority_bucket) = &toml.priority_bucket {
            self.priority_bucket = priority_bucket.clone();
        }
        if let Some(bootstrap_ascending) = &toml.bootstrap_ascending {
            self.bootstrap_ascending = bootstrap_ascending.clone();
        }
        if let Some(bootstrap_server) = &toml.bootstrap_server {
            self.bootstrap_server = bootstrap_server.clone();
        }
        if let Some(secondary_work_peers) = &toml.secondary_work_peers {
            self.secondary_work_peers = secondary_work_peers.clone();
        }
        if let Some(max_pruning_age_s) = toml.max_pruning_age_s {
            self.max_pruning_age_s = max_pruning_age_s;
        }
        if let Some(max_pruning_depth) = toml.max_pruning_depth {
            self.max_pruning_depth = max_pruning_depth;
        }
        if let Some(callback_address) = &toml.callback_address {
            self.callback_address = callback_address.clone();
        }
        if let Some(callback_port) = toml.callback_port {
            self.callback_port = callback_port;
        }
        if let Some(callback_target) = &toml.callback_target {
            self.callback_target = callback_target.clone();
        }
        if let Some(websocket_config) = &toml.websocket_config {
            self.websocket_config = websocket_config.clone();
        }
        if let Some(ipc_config) = &toml.ipc_config {
            self.ipc_config = ipc_config.clone();
        }
        if let Some(diagnostics_config) = &toml.diagnostics_config {
            self.diagnostics_config = diagnostics_config.clone();
        }
        if let Some(stat_config) = &toml.stat_config {
            self.stat_config = stat_config.clone();
        }
        if let Some(lmdb_config) = &toml.lmdb_config {
            self.lmdb_config = lmdb_config.clone();
        }
        if let Some(backlog_scan_batch_size) = toml.backlog_scan_batch_size {
            self.backlog_scan_batch_size = backlog_scan_batch_size;
        }
        if let Some(backlog_scan_frequency) = toml.backlog_scan_frequency {
            self.backlog_scan_frequency = backlog_scan_frequency;
        }
        if let Some(vote_cache) = &toml.vote_cache {
            self.vote_cache = vote_cache.clone();
        }
        if let Some(rep_crawler_query_timeout) = &toml.rep_crawler_query_timeout {
            self.rep_crawler_query_timeout =
                Duration::from_millis(rep_crawler_query_timeout.0 as u64);
        }
        if let Some(block_processor) = &toml.block_processor {
            self.block_processor = block_processor.clone();
        }
        if let Some(active_elections) = &toml.active_elections {
            self.active_elections = active_elections.clone();
        }
        if let Some(vote_processor) = &toml.vote_processor {
            self.vote_processor = vote_processor.clone();
        }
        if let Some(tcp) = &toml.tcp {
            self.tcp = tcp.clone();
        }
        if let Some(request_aggregator) = &toml.request_aggregator {
            self.request_aggregator = request_aggregator.clone();
        }
        if let Some(message_processor) = &toml.message_processor {
            self.message_processor = message_processor.clone();
        }
        if let Some(priority_scheduler_enabled) = toml.priority_scheduler_enabled {
            self.priority_scheduler_enabled = priority_scheduler_enabled;
        }
        if let Some(local_block_broadcaster) = &toml.local_block_broadcaster {
            self.local_block_broadcaster = local_block_broadcaster.clone();
        }
        if let Some(confirming_set) = &toml.confirming_set {
            self.confirming_set = confirming_set.clone();
        }
        if let Some(monitor) = &toml.monitor {
            self.monitor = monitor.clone();
        }
    }

    pub fn new_test_instance() -> Self {
        Self::default(None, &DEV_NETWORK_PARAMS, 1)
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        if let Some(port) = self.peering_port {
            toml.put_u16("peering_port", port, "Node peering port.\ntype:uint16")?;
        }

        toml.put_u32("bootstrap_fraction_numerator", self.bootstrap_fraction_numerator, "Change bootstrap threshold (online stake / 256 * bootstrap_fraction_numerator).\ntype:uint32")?;
        toml.put_str("receive_minimum", &self.receive_minimum.to_string_dec (), "Minimum receive amount. Only affects node wallets. A large amount is recommended to avoid automatic work generation for tiny transactions.\ntype:string,amount,raw")?;
        toml.put_str("online_weight_minimum", &self.online_weight_minimum.to_string_dec (), "When calculating online weight, the node is forced to assume at least this much voting weight is online, thus setting a floor for voting weight to confirm transactions at online_weight_minimum * \"quorum delta\".\ntype:string,amount,raw")?;
        toml.put_str("representative_vote_weight_minimum", &self.representative_vote_weight_minimum.to_string_dec(), "Minimum vote weight that a representative must have for its vote to be counted.\nAll representatives above this weight will be kept in memory!\ntype:string,amount,raw")?;
        toml.put_u32(
            "password_fanout",
            self.password_fanout,
            "Password fanout factor.\ntype:uint64",
        )?;
        toml.put_u32("io_threads", self.io_threads, "Number of threads dedicated to I/O operations. Defaults to the number of CPU threads, and at least 4.\ntype:uint64")?;
        toml.put_u32("network_threads", self.network_threads, "Number of threads dedicated to processing network messages. Defaults to the number of CPU threads, and at least 4.\ntype:uint64")?;
        toml.put_u32("work_threads", self.work_threads, "Number of threads dedicated to CPU generated work. Defaults to all available CPU threads.\ntype:uint64")?;
        toml.put_u32("background_threads", self.background_threads, "Number of threads dedicated to background node work, including handling of RPC requests. Defaults to all available CPU threads.\ntype:uint64")?;
        toml.put_u32("signature_checker_threads", self.signature_checker_threads, "Number of additional threads dedicated to signature verification. Defaults to number of CPU threads / 2.\ntype:uint64")?;
        toml.put_bool("enable_voting", self.enable_voting, "Enable or disable voting. Enabling this option requires additional system resources, namely increased CPU, bandwidth and disk usage.\ntype:bool")?;
        toml.put_u32("bootstrap_connections", self.bootstrap_connections, "Number of outbound bootstrap connections. Must be a power of 2. Defaults to 4.\nWarning: a larger amount of connections may use substantially more system memory.\ntype:uint64")?;
        toml.put_u32("bootstrap_connections_max", self.bootstrap_connections_max, "Maximum number of inbound bootstrap connections. Defaults to 64.\nWarning: a larger amount of connections may use additional system memory.\ntype:uint64")?;
        toml.put_u32("bootstrap_initiator_threads", self.bootstrap_initiator_threads, "Number of threads dedicated to concurrent bootstrap attempts. Defaults to 1.\nWarning: a larger amount of attempts may use additional system memory and disk IO.\ntype:uint64")?;
        toml.put_u32("bootstrap_serving_threads", self.bootstrap_serving_threads, "Number of threads dedicated to serving bootstrap data to other peers. Defaults to half the number of CPU threads, and at least 2.\ntype:uint64")?;
        toml.put_u32("bootstrap_frontier_request_count", self.bootstrap_frontier_request_count, "Number frontiers per bootstrap frontier request. Defaults to 1048576.\ntype:uint32,[1024..4294967295]")?;
        toml.put_i64("block_processor_batch_max_time", self.block_processor_batch_max_time_ms, "The maximum time the block processor can continuously process blocks for.\ntype:milliseconds")?;
        toml.put_bool(
            "allow_local_peers",
            self.allow_local_peers,
            "Enable or disable local host peering.\ntype:bool",
        )?;
        toml.put_str("vote_minimum", &self.vote_minimum.to_string_dec (), "Local representatives do not vote if the delegated weight is under this threshold. Saves on system resources.\ntype:string,amount,raw")?;
        toml.put_i64("vote_generator_delay", self.vote_generator_delay_ms, "Delay before votes are sent to allow for efficient bundling of hashes in votes.\ntype:milliseconds")?;
        toml.put_u32("vote_generator_threshold", self.vote_generator_threshold, "Number of bundled hashes required for an additional generator delay.\ntype:uint64,[1..11]")?;
        toml.put_i64("unchecked_cutoff_time", self.unchecked_cutoff_time_s, "Number of seconds before deleting an unchecked entry.\nWarning: lower values (e.g., 3600 seconds, or 1 hour) may result in unsuccessful bootstraps, especially a bootstrap from scratch.\ntype:seconds")?;
        toml.put_i64("tcp_io_timeout", self.tcp_io_timeout_s , "Timeout for TCP connect-, read- and write operations.\nWarning: a low value (e.g., below 5 seconds) may result in TCP connections failing.\ntype:seconds")?;
        toml.put_i64 ("pow_sleep_interval", self.pow_sleep_interval_ns, "Time to sleep between batch work generation attempts. Reduces max CPU usage at the expense of a longer generation time.\ntype:nanoseconds")?;
        toml.put_str("external_address", &self.external_address, "The external address of this node (NAT). If not set, the node will request this information via UPnP.\ntype:string,ip")?;
        toml.put_u16("external_port", self.external_port, "The external port number of this node (NAT). Only used if external_address is set.\ntype:uint16")?;
        toml.put_u32(
            "tcp_incoming_connections_max",
            self.tcp_incoming_connections_max,
            "Maximum number of incoming TCP connections.\ntype:uint64",
        )?;
        toml.put_bool("use_memory_pools", self.use_memory_pools, "If true, allocate memory from memory pools. Enabling this may improve performance. Memory is never released to the OS.\ntype:bool")?;

        toml.put_usize("bandwidth_limit", self.bandwidth_limit, "Outbound traffic limit in bytes/sec after which messages will be dropped.\nNote: changing to unlimited bandwidth (0) is not recommended for limited connections.\ntype:uint64")?;
        toml.put_f64(
            "bandwidth_limit_burst_ratio",
            self.bandwidth_limit_burst_ratio,
            "Burst ratio for outbound traffic shaping.\ntype:double",
        )?;

        toml.put_usize("bootstrap_bandwidth_limit", self.bootstrap_bandwidth_limit, "Outbound bootstrap traffic limit in bytes/sec after which messages will be dropped.\nNote: changing to unlimited bandwidth (0) is not recommended for limited connections.\ntype:uint64")?;
        toml.put_f64(
            "bootstrap_bandwidth_burst_ratio",
            self.bootstrap_bandwidth_burst_ratio,
            "Burst ratio for outbound bootstrap traffic.\ntype:double",
        )?;

        toml.put_i64("confirming_set_batch_time", self.confirming_set_batch_time.as_millis() as i64, "Maximum time the confirming set will hold the database write transaction.\ntype:milliseconds")?;
        toml.put_bool("backup_before_upgrade", self.backup_before_upgrade, "Backup the ledger database before performing upgrades.\nWarning: uses more disk storage and increases startup time when upgrading.\ntype:bool")?;
        toml.put_f64(
            "max_work_generate_multiplier",
            self.max_work_generate_multiplier,
            "Maximum allowed difficulty multiplier for work generation.\ntype:double,[1..]",
        )?;

        toml.put_str(
            "frontiers_confirmation",
            serialize_frontiers_confirmation(self.frontiers_confirmation),
            "Mode controlling frontier confirmation rate.\ntype:string,{auto,always,disabled}",
        )?;
        toml.put_u32("max_queued_requests", self.max_queued_requests, "Limit for number of queued confirmation requests for one channel, after which new requests are dropped until the queue drops below this value.\ntype:uint32")?;
        toml.put_u32("request_aggregator_threads", self.request_aggregator_threads, "Number of threads to dedicate to request aggregator. Defaults to using all cpu threads, up to a maximum of 4")?;
        toml.put_str("rep_crawler_weight_minimum", &self.rep_crawler_weight_minimum.to_string_dec (), "Rep crawler minimum weight, if this is less than minimum principal weight then this is taken as the minimum weight a rep must have to be tracked. If you want to track all reps set this to 0. If you do not want this to influence anything then set it to max value. This is only useful for debugging or for people who really know what they are doing.\ntype:string,amount,raw")?;

        toml.put_u32 ("backlog_scan_batch_size", self.backlog_scan_batch_size, "Number of accounts per second to process when doing backlog population scan. Increasing this value will help unconfirmed frontiers get into election prioritization queue faster, however it will also increase resource usage. \ntype:uint")?;
        toml.put_u32 ("backlog_scan_frequency", self.backlog_scan_frequency, "Backlog scan divides the scan into smaller batches, number of which is controlled by this value. Higher frequency helps to utilize resources more uniformly, however it also introduces more overhead. The resulting number of accounts per single batch is `backlog_scan_batch_size / backlog_scan_frequency` \ntype:uint")?;

        toml.create_array(
            "work_peers",
            "A list of \"address:port\" entries to identify work peers.",
            &mut |work_peers| {
                for peer in &self.work_peers {
                    work_peers.push_back_str(&format!("{}:{}", peer.address, peer.port))?;
                }
                Ok(())
            },
        )?;

        toml.create_array ("preconfigured_peers", "A list of \"address\" (hostname or ipv6 notation ip address) entries to identify preconfigured peers.\nThe contents of the NANO_DEFAULT_PEER environment variable are added to preconfigured_peers.",
            &mut |peers| {
                for peer in &self.preconfigured_peers {
                    peers.push_back_str(peer)?;
                }
                Ok(())
            })?;

        toml.create_array ("preconfigured_representatives", "A list of representative account addresses used when creating new accounts in internal wallets.",
            &mut |reps|{
                for rep in &self.preconfigured_representatives {
                    reps.push_back_str(&rep.encode_account())?;
                }
                Ok(())
            })?;

        toml.put_child("experimental", &mut|child|{
                child.create_array ("secondary_work_peers", "A list of \"address:port\" entries to identify work peers for secondary work generation.",
            &mut |peers|{
                for p in &self.secondary_work_peers{
                    peers.push_back_str(&format!("{}:{}", p.address, p.port))?;
                }
                Ok(())
            })?;
                child.put_i64("max_pruning_age", self.max_pruning_age_s, "Time limit for blocks age after pruning.\ntype:seconds")?;
                child.put_u64("max_pruning_depth", self.max_pruning_depth, "Limit for full blocks in chain after pruning.\ntype:uint64")?;
                Ok(())
            })?;

        toml.put_child("httpcallback", &mut |callback| {
            callback.put_str(
                "address",
                &self.callback_address,
                "Callback address.\ntype:string,ip",
            )?;
            callback.put_u16(
                "port",
                self.callback_port,
                "Callback port number.\ntype:uint16",
            )?;
            callback.put_str(
                "target",
                &self.callback_target,
                "Callback target path.\ntype:string,uri",
            )?;
            Ok(())
        })?;

        toml.put_child("websocket", &mut |websocket| {
            self.websocket_config.serialize_toml(websocket)
        })?;

        toml.put_child("ipc", &mut |ipc| self.ipc_config.serialize_toml(ipc))?;

        toml.put_child("diagnostics", &mut |diagnostics| {
            self.diagnostics_config.serialize_toml(diagnostics)
        })?;

        toml.put_child("statistics", &mut |statistics| {
            self.stat_config.serialize_toml(statistics)
        })?;

        toml.put_child("lmdb", &mut |lmdb| self.lmdb_config.serialize_toml(lmdb))?;

        toml.put_child("optimistic_scheduler", &mut |opt| {
            self.optimistic_scheduler.serialize_toml(opt)
        })?;

        toml.put_child("priority_bucket", &mut |opt| {
            self.priority_bucket.serialize_toml(opt)
        })?;

        toml.put_child("bootstrap_ascending", &mut |writer| {
            self.bootstrap_ascending.serialize_toml(writer)
        })?;

        toml.put_child("bootstrap_server", &mut |writer| {
            self.bootstrap_server.serialize_toml(writer)
        })?;

        toml.put_child("vote_cache", &mut |writer| {
            self.vote_cache.serialize_toml(writer)
        })?;

        toml.put_child("rep_crawler", &mut |writer| {
            writer.put_u64(
                "query_timeout",
                self.rep_crawler_query_timeout.as_millis() as u64,
                "",
            )
        })?;

        toml.put_child("active_elections", &mut |writer| {
            self.active_elections.serialize_toml(writer)
        })?;

        toml.put_child("block_processor", &mut |writer| {
            self.block_processor.serialize_toml(writer)
        })?;

        toml.put_child("vote_processor", &mut |writer| {
            self.vote_processor.serialize_toml(writer)
        })?;

        toml.put_child("request_aggregator", &mut |writer| {
            self.request_aggregator.serialize_toml(writer)
        })?;

        toml.put_child("message_processor", &mut |writer| {
            self.message_processor.serialize_toml(writer)
        })?;

        toml.put_child("monitor", &mut |writer| self.monitor.serialize_toml(writer))?;

        Ok(())
    }

    pub fn random_representative(&self) -> Account {
        let i = thread_rng().gen_range(0..self.preconfigured_representatives.len());
        return self.preconfigured_representatives[i];
    }
}

fn serialize_frontiers_confirmation(mode: FrontiersConfirmationMode) -> &'static str {
    match mode {
        FrontiersConfirmationMode::Always => "always",
        FrontiersConfirmationMode::Automatic => "auto",
        FrontiersConfirmationMode::Disabled => "disabled",
        FrontiersConfirmationMode::Invalid => "auto",
    }
}

impl From<NodeConfigToml> for NodeConfig {
    fn from(toml: NodeConfigToml) -> Self {
        Self {
            peering_port: toml.peering_port,
            optimistic_scheduler: toml.optimistic_scheduler.unwrap(),
            hinted_scheduler: toml.hinted_scheduler.unwrap(),
            priority_bucket: toml.priority_bucket.unwrap(),
            bootstrap_fraction_numerator: toml.bootstrap_fraction_numerator.unwrap(),
            receive_minimum: toml.receive_minimum.unwrap(),
            online_weight_minimum: toml.online_weight_minimum.unwrap(),
            representative_vote_weight_minimum: toml.representative_vote_weight_minimum.unwrap(),
            password_fanout: toml.password_fanout.unwrap(),
            io_threads: toml.io_threads.unwrap(),
            network_threads: toml.network_threads.unwrap(),
            work_threads: toml.work_threads.unwrap(),
            background_threads: toml.background_threads.unwrap(),
            signature_checker_threads: toml.signature_checker_threads.unwrap(),
            enable_voting: toml.enable_voting.unwrap(),
            bootstrap_connections: toml.bootstrap_connections.unwrap(),
            bootstrap_connections_max: toml.bootstrap_connections_max.unwrap(),
            bootstrap_initiator_threads: toml.bootstrap_initiator_threads.unwrap(),
            bootstrap_serving_threads: toml.bootstrap_serving_threads.unwrap(),
            bootstrap_frontier_request_count: toml.bootstrap_frontier_request_count.unwrap(),
            block_processor_batch_max_time_ms: toml.block_processor_batch_max_time_ms.unwrap(),
            allow_local_peers: toml.allow_local_peers.unwrap(),
            vote_minimum: toml.vote_minimum.unwrap(),
            vote_generator_delay_ms: toml.vote_generator_delay_ms.unwrap(),
            vote_generator_threshold: toml.vote_generator_threshold.unwrap(),
            unchecked_cutoff_time_s: toml.unchecked_cutoff_time_s.unwrap(),
            tcp_io_timeout_s: toml.tcp_io_timeout_s.unwrap(),
            pow_sleep_interval_ns: toml.pow_sleep_interval_ns.unwrap(),
            external_address: toml.external_address.unwrap(),
            external_port: toml.external_port.unwrap(),
            tcp_incoming_connections_max: toml.tcp_incoming_connections_max.unwrap(),
            use_memory_pools: toml.use_memory_pools.unwrap(),
            bandwidth_limit: toml.bandwidth_limit.unwrap(),
            bandwidth_limit_burst_ratio: toml.bandwidth_limit_burst_ratio.unwrap(),
            bootstrap_ascending: toml.bootstrap_ascending.unwrap(),
            bootstrap_server: toml.bootstrap_server.unwrap(),
            bootstrap_bandwidth_limit: toml.bootstrap_bandwidth_limit.unwrap(),
            bootstrap_bandwidth_burst_ratio: toml.bootstrap_bandwidth_burst_ratio.unwrap(),
            confirming_set_batch_time: Duration::from_millis(
                toml.confirming_set_batch_time.unwrap().0 as u64,
            ),
            backup_before_upgrade: toml.backup_before_upgrade.unwrap(),
            max_work_generate_multiplier: toml.max_work_generate_multiplier.unwrap(),
            frontiers_confirmation: toml.frontiers_confirmation.unwrap(),
            max_queued_requests: toml.max_queued_requests.unwrap(),
            request_aggregator_threads: toml.request_aggregator_threads.unwrap(),
            max_unchecked_blocks: toml.max_unchecked_blocks.unwrap(),
            rep_crawler_weight_minimum: toml.rep_crawler_weight_minimum.unwrap(),
            work_peers: toml.work_peers.unwrap(),
            secondary_work_peers: toml.secondary_work_peers.unwrap(),
            preconfigured_peers: toml.preconfigured_peers.unwrap(),
            preconfigured_representatives: toml.preconfigured_representatives.unwrap(),
            max_pruning_age_s: toml.max_pruning_age_s.unwrap(),
            max_pruning_depth: toml.max_pruning_depth.unwrap(),
            callback_address: toml.callback_address.unwrap(),
            callback_port: toml.callback_port.unwrap(),
            callback_target: toml.callback_target.unwrap(),
            websocket_config: toml.websocket_config.unwrap(),
            ipc_config: toml.ipc_config.unwrap(),
            diagnostics_config: toml.diagnostics_config.unwrap(),
            stat_config: toml.stat_config.unwrap(),
            lmdb_config: toml.lmdb_config.unwrap(),
            backlog_scan_batch_size: toml.backlog_scan_batch_size.unwrap(),
            backlog_scan_frequency: toml.backlog_scan_frequency.unwrap(),
            vote_cache: toml.vote_cache.unwrap(),
            rep_crawler_query_timeout: Duration::from_millis(
                toml.rep_crawler_query_timeout.unwrap().0 as u64,
            ),
            block_processor: toml.block_processor.unwrap(),
            active_elections: toml.active_elections.unwrap(),
            vote_processor: toml.vote_processor.unwrap(),
            tcp: toml.tcp.unwrap(),
            request_aggregator: toml.request_aggregator.unwrap(),
            message_processor: toml.message_processor.unwrap(),
            priority_scheduler_enabled: toml.priority_scheduler_enabled.unwrap(),
            local_block_broadcaster: toml.local_block_broadcaster.unwrap(),
            confirming_set: toml.confirming_set.unwrap(),
            monitor: toml.monitor.unwrap(),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct MonitorConfig {
    pub enabled: bool,
    pub interval: Duration,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(60),
        }
    }
}

impl MonitorConfig {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_bool(
            "enable",
            self.enabled,
            "Enable or disable periodic node status logging\ntype:bool",
        )?;

        toml.put_u64(
            "interval",
            self.interval.as_secs(),
            "Interval between status logs\ntype:seconds",
        )
    }
}
