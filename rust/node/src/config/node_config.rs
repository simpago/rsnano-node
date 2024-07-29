use crate::block_processing::BlockProcessorConfig;
use crate::bootstrap::{BootstrapAscendingConfig, BootstrapServerConfig};
use crate::consensus::{
    ActiveElectionsConfig, HintedSchedulerConfig, OptimisticSchedulerConfig, PriorityBucketConfig,
    RequestAggregatorConfig, VoteCacheConfig, VoteProcessorConfig,
};
use crate::monitor::MonitorConfig;
use crate::stats::StatsConfig;
use crate::transport::MessageProcessorConfig;
use crate::utils::TxnTrackingConfig;
use crate::websocket::WebsocketConfig;
use crate::IpcConfig;
use crate::{
    block_processing::LocalBlockBroadcasterConfig, bootstrap::BootstrapInitiatorConfig,
    cementation::ConfirmingSetConfig, transport::TcpConfig, NetworkParams, DEV_NETWORK_PARAMS,
};
use once_cell::sync::Lazy;
use rand::{thread_rng, Rng};
use rsnano_core::utils::{get_cpu_count, get_env_or_default_string, is_sanitizer_build};
use rsnano_core::{Account, Amount, Networks, GXRB_RATIO, XRB_RATIO};
use rsnano_store_lmdb::LmdbConfig;
use serde::Serialize;
use serde::{Deserialize, Deserializer, Serializer};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use std::{cmp::max, net::Ipv6Addr};

use super::{Miliseconds, NodeConfigToml};

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
    pub txn_tracking_config: TxnTrackingConfig,
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

impl Default for NodeConfig {
    fn default() -> Self {
        let network_params = &NetworkParams::default();
        Self::new(
            Some(network_params.network.default_node_port),
            network_params,
            get_cpu_count(),
        )
    }
}

#[derive(Clone)]
pub struct Peer {
    pub address: String,
    pub port: u16,
}

impl fmt::Display for Peer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.address, self.port)
    }
}

impl FromStr for Peer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid format".into());
        }

        let address = parts[0].to_string();
        let port = parts[1]
            .parse::<u16>()
            .map_err(|_| "Invalid port".to_string())?;

        Ok(Peer { address, port })
    }
}

impl Serialize for Peer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Peer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Peer>().map_err(serde::de::Error::custom)
    }
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
    pub fn new(
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
            txn_tracking_config: TxnTrackingConfig::new(),
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
            block_processor: BlockProcessorConfig::default(),
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

    pub fn new_test_instance() -> Self {
        Self::new(None, &DEV_NETWORK_PARAMS, 1)
    }

    pub fn random_representative(&self) -> Account {
        let i = thread_rng().gen_range(0..self.preconfigured_representatives.len());
        return self.preconfigured_representatives[i];
    }
}

impl From<NodeConfig> for NodeConfigToml {
    fn from(node_config: NodeConfig) -> Self {
        Self {
            allow_local_peers: Some(node_config.allow_local_peers),
            background_threads: Some(node_config.background_threads),
            backlog_scan_batch_size: Some(node_config.backlog_scan_batch_size),
            backlog_scan_frequency: Some(node_config.backlog_scan_frequency),
            backup_before_upgrade: Some(node_config.backup_before_upgrade),
            bandwidth_limit: Some(node_config.bandwidth_limit),
            bandwidth_limit_burst_ratio: Some(node_config.bandwidth_limit_burst_ratio),
            block_processor_batch_max_time_ms: Some(node_config.block_processor_batch_max_time_ms),
            bootstrap_bandwidth_burst_ratio: Some(node_config.bootstrap_bandwidth_burst_ratio),
            bootstrap_bandwidth_limit: Some(node_config.bootstrap_bandwidth_limit),
            bootstrap_connections: Some(node_config.bootstrap_connections),
            bootstrap_connections_max: Some(node_config.bootstrap_connections_max),
            bootstrap_fraction_numerator: Some(node_config.bootstrap_fraction_numerator),
            bootstrap_frontier_request_count: Some(node_config.bootstrap_frontier_request_count),
            bootstrap_initiator_threads: Some(node_config.bootstrap_initiator_threads),
            bootstrap_serving_threads: Some(node_config.bootstrap_serving_threads),
            confirming_set_batch_time: Some(Miliseconds(
                node_config.confirming_set_batch_time.as_millis(),
            )),
            enable_voting: Some(node_config.enable_voting),
            external_address: Some(node_config.external_address.clone()),
            external_port: Some(node_config.external_port),
            frontiers_confirmation: Some(node_config.frontiers_confirmation),
            io_threads: Some(node_config.io_threads),
            max_queued_requests: Some(node_config.max_queued_requests),
            max_unchecked_blocks: Some(node_config.max_unchecked_blocks),
            max_work_generate_multiplier: Some(node_config.max_work_generate_multiplier),
            network_threads: Some(node_config.network_threads),
            online_weight_minimum: Some(node_config.online_weight_minimum),
            password_fanout: Some(node_config.password_fanout),
            peering_port: node_config.peering_port,
            pow_sleep_interval_ns: Some(node_config.pow_sleep_interval_ns),
            preconfigured_peers: Some(node_config.preconfigured_peers.clone()),
            preconfigured_representatives: Some(node_config.preconfigured_representatives.clone()),
            receive_minimum: Some(node_config.receive_minimum),
            rep_crawler_weight_minimum: Some(node_config.rep_crawler_weight_minimum),
            representative_vote_weight_minimum: Some(
                node_config.representative_vote_weight_minimum,
            ),
            request_aggregator_threads: Some(node_config.request_aggregator_threads),
            signature_checker_threads: Some(node_config.signature_checker_threads),
            tcp_incoming_connections_max: Some(node_config.tcp_incoming_connections_max),
            tcp_io_timeout_s: Some(node_config.tcp_io_timeout_s),
            unchecked_cutoff_time_s: Some(node_config.unchecked_cutoff_time_s),
            use_memory_pools: Some(node_config.use_memory_pools),
            vote_generator_delay_ms: Some(node_config.vote_generator_delay_ms),
            vote_generator_threshold: Some(node_config.vote_generator_threshold),
            vote_minimum: Some(node_config.vote_minimum),
            work_peers: Some(node_config.work_peers),
            work_threads: Some(node_config.work_threads),
            optimistic_scheduler: Some((&node_config.optimistic_scheduler).into()),
            priority_bucket: Some((&node_config.priority_bucket).into()),
            bootstrap_ascending: Some((&node_config.bootstrap_ascending).into()),
            bootstrap_server: Some((&node_config.bootstrap_server).into()),
            secondary_work_peers: Some(node_config.secondary_work_peers),
            max_pruning_age_s: Some(node_config.max_pruning_age_s),
            max_pruning_depth: Some(node_config.max_pruning_depth),
            websocket_config: Some(node_config.websocket_config.into()),
            ipc_config: Some(node_config.ipc_config.into()),
            txn_tracking_config: Some((&node_config.txn_tracking_config).into()),
            stat_config: Some(node_config.stat_config.into()),
            lmdb_config: Some((&node_config.lmdb_config).into()),
            vote_cache: Some((&node_config.vote_cache).into()),
            block_processor: Some((&node_config.block_processor).into()),
            active_elections: Some((&node_config.active_elections).into()),
            vote_processor: Some((&node_config.vote_processor).into()),
            request_aggregator: Some((&node_config.request_aggregator).into()),
            message_processor: Some((&node_config.message_processor).into()),
            monitor: Some((&node_config.monitor).into()),
            callback_address: Some(node_config.callback_address),
            callback_port: Some(node_config.callback_port),
            callback_target: Some(node_config.callback_target),
        }
    }
}
