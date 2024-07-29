mod active_elections_config_toml;
mod block_processor_config_toml;
mod bootstrap_ascending_config_toml;
mod bootstrap_server_config_toml;
mod ipc_config_toml;
mod lmdb_config_toml;
mod message_processor_config_toml;
mod monitor_config_toml;
mod optimistic_scheduler_config_toml;
mod priority_bucket_config_toml;
mod request_aggregator_config_toml;
mod stats_config_toml;
mod txn_tracking_config_toml;
mod vote_cache_config_toml;
mod vote_processor_config_toml;
mod websocket_config_toml;

pub use active_elections_config_toml::*;
pub use block_processor_config_toml::*;
pub use bootstrap_ascending_config_toml::*;
pub use bootstrap_server_config_toml::*;
pub use ipc_config_toml::*;
pub use lmdb_config_toml::*;
pub use message_processor_config_toml::*;
pub use monitor_config_toml::*;
pub use optimistic_scheduler_config_toml::*;
pub use priority_bucket_config_toml::*;
pub use request_aggregator_config_toml::*;
pub use stats_config_toml::*;
pub use txn_tracking_config_toml::*;
pub use vote_cache_config_toml::*;
pub use vote_processor_config_toml::*;
pub use websocket_config_toml::*;

use crate::config::{FrontiersConfirmationMode, Miliseconds, NodeConfig, Peer};
use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct NodeConfigToml {
    pub allow_local_peers: Option<bool>,
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
    pub(crate) optimistic_scheduler: Option<OptimisticSchedulerConfigToml>,
    pub(crate) priority_bucket: Option<PriorityBucketConfigToml>,
    pub(crate) bootstrap_ascending: Option<BootstrapAscendingConfigToml>,
    pub(crate) bootstrap_server: Option<BootstrapServerConfigToml>,
    pub(crate) secondary_work_peers: Option<Vec<Peer>>,
    pub(crate) max_pruning_age_s: Option<i64>,
    pub(crate) max_pruning_depth: Option<u64>,
    pub(crate) websocket_config: Option<WebsocketConfigToml>,
    pub(crate) ipc_config: Option<IpcConfigToml>,
    pub(crate) txn_tracking_config: Option<TxnTrackingConfigToml>,
    pub(crate) stat_config: Option<StatsConfigToml>,
    pub(crate) lmdb_config: Option<LmdbConfigToml>,
    pub(crate) vote_cache: Option<VoteCacheConfigToml>,
    pub(crate) block_processor: Option<BlockProcessorConfigToml>,
    pub(crate) active_elections: Option<ActiveElectionsConfigToml>,
    pub(crate) vote_processor: Option<VoteProcessorConfigToml>,
    pub(crate) request_aggregator: Option<RequestAggregatorConfigToml>,
    pub(crate) message_processor: Option<MessageProcessorConfigToml>,
    pub(crate) monitor: Option<MonitorConfigToml>,
    pub(crate) callback_address: Option<String>,
    pub(crate) callback_port: Option<u16>,
    pub(crate) callback_target: Option<String>,
}

impl From<NodeConfigToml> for NodeConfig {
    fn from(toml: NodeConfigToml) -> Self {
        let mut config = NodeConfig::default();

        if let Some(allow_local_peers) = toml.allow_local_peers {
            config.allow_local_peers = allow_local_peers;
        }
        if let Some(background_threads) = toml.background_threads {
            config.background_threads = background_threads;
        }
        if let Some(backlog_scan_batch_size) = toml.backlog_scan_batch_size {
            config.backlog_scan_batch_size = backlog_scan_batch_size;
        }
        if let Some(backlog_scan_frequency) = toml.backlog_scan_frequency {
            config.backlog_scan_frequency = backlog_scan_frequency;
        }
        if let Some(backup_before_upgrade) = toml.backup_before_upgrade {
            config.backup_before_upgrade = backup_before_upgrade;
        }
        if let Some(bandwidth_limit) = toml.bandwidth_limit {
            config.bandwidth_limit = bandwidth_limit;
        }
        if let Some(bandwidth_limit_burst_ratio) = toml.bandwidth_limit_burst_ratio {
            config.bandwidth_limit_burst_ratio = bandwidth_limit_burst_ratio;
        }
        if let Some(block_processor_batch_max_time_ms) = toml.block_processor_batch_max_time_ms {
            config.block_processor_batch_max_time_ms = block_processor_batch_max_time_ms;
        }
        if let Some(bootstrap_bandwidth_burst_ratio) = toml.bootstrap_bandwidth_burst_ratio {
            config.bootstrap_bandwidth_burst_ratio = bootstrap_bandwidth_burst_ratio;
        }
        if let Some(bootstrap_bandwidth_limit) = toml.bootstrap_bandwidth_limit {
            config.bootstrap_bandwidth_limit = bootstrap_bandwidth_limit;
        }
        if let Some(bootstrap_connections) = toml.bootstrap_connections {
            config.bootstrap_connections = bootstrap_connections;
        }
        if let Some(bootstrap_connections_max) = toml.bootstrap_connections_max {
            config.bootstrap_connections_max = bootstrap_connections_max;
        }
        if let Some(bootstrap_fraction_numerator) = toml.bootstrap_fraction_numerator {
            config.bootstrap_fraction_numerator = bootstrap_fraction_numerator;
        }
        if let Some(bootstrap_frontier_request_count) = toml.bootstrap_frontier_request_count {
            config.bootstrap_frontier_request_count = bootstrap_frontier_request_count;
        }
        if let Some(bootstrap_initiator_threads) = toml.bootstrap_initiator_threads {
            config.bootstrap_initiator_threads = bootstrap_initiator_threads;
        }
        if let Some(bootstrap_serving_threads) = toml.bootstrap_serving_threads {
            config.bootstrap_serving_threads = bootstrap_serving_threads;
        }
        if let Some(confirming_set_batch_time) = &toml.confirming_set_batch_time {
            config.confirming_set_batch_time =
                Duration::from_millis(confirming_set_batch_time.0 as u64);
        }
        if let Some(enable_voting) = toml.enable_voting {
            config.enable_voting = enable_voting;
        }
        if let Some(external_address) = &toml.external_address {
            config.external_address = external_address.clone();
        }
        if let Some(external_port) = toml.external_port {
            config.external_port = external_port;
        }
        if let Some(frontiers_confirmation) = toml.frontiers_confirmation {
            config.frontiers_confirmation = frontiers_confirmation;
        }
        if let Some(io_threads) = toml.io_threads {
            config.io_threads = io_threads;
        }
        if let Some(max_queued_requests) = toml.max_queued_requests {
            config.max_queued_requests = max_queued_requests;
        }
        if let Some(max_unchecked_blocks) = toml.max_unchecked_blocks {
            config.max_unchecked_blocks = max_unchecked_blocks;
        }
        if let Some(max_work_generate_multiplier) = toml.max_work_generate_multiplier {
            config.max_work_generate_multiplier = max_work_generate_multiplier;
        }
        if let Some(network_threads) = toml.network_threads {
            config.network_threads = network_threads;
        }
        if let Some(online_weight_minimum) = toml.online_weight_minimum {
            config.online_weight_minimum = online_weight_minimum;
        }
        if let Some(password_fanout) = toml.password_fanout {
            config.password_fanout = password_fanout;
        }
        if let Some(peering_port) = toml.peering_port {
            config.peering_port = Some(peering_port);
        }
        if let Some(pow_sleep_interval_ns) = toml.pow_sleep_interval_ns {
            config.pow_sleep_interval_ns = pow_sleep_interval_ns;
        }
        if let Some(preconfigured_peers) = &toml.preconfigured_peers {
            config.preconfigured_peers = preconfigured_peers.clone();
        }
        if let Some(preconfigured_representatives) = &toml.preconfigured_representatives {
            config.preconfigured_representatives = preconfigured_representatives.clone();
        }
        if let Some(receive_minimum) = toml.receive_minimum {
            config.receive_minimum = receive_minimum;
        }
        if let Some(rep_crawler_weight_minimum) = toml.rep_crawler_weight_minimum {
            config.rep_crawler_weight_minimum = rep_crawler_weight_minimum;
        }
        if let Some(representative_vote_weight_minimum) = toml.representative_vote_weight_minimum {
            config.representative_vote_weight_minimum = representative_vote_weight_minimum;
        }
        if let Some(request_aggregator_threads) = toml.request_aggregator_threads {
            config.request_aggregator_threads = request_aggregator_threads;
        }
        if let Some(signature_checker_threads) = toml.signature_checker_threads {
            config.signature_checker_threads = signature_checker_threads;
        }
        if let Some(tcp_incoming_connections_max) = toml.tcp_incoming_connections_max {
            config.tcp_incoming_connections_max = tcp_incoming_connections_max;
        }
        if let Some(tcp_io_timeout_s) = toml.tcp_io_timeout_s {
            config.tcp_io_timeout_s = tcp_io_timeout_s;
        }
        if let Some(unchecked_cutoff_time_s) = toml.unchecked_cutoff_time_s {
            config.unchecked_cutoff_time_s = unchecked_cutoff_time_s;
        }
        if let Some(use_memory_pools) = toml.use_memory_pools {
            config.use_memory_pools = use_memory_pools;
        }
        if let Some(vote_generator_delay_ms) = toml.vote_generator_delay_ms {
            config.vote_generator_delay_ms = vote_generator_delay_ms;
        }
        if let Some(vote_generator_threshold) = toml.vote_generator_threshold {
            config.vote_generator_threshold = vote_generator_threshold;
        }
        if let Some(vote_minimum) = toml.vote_minimum {
            config.vote_minimum = vote_minimum;
        }
        if let Some(work_peers) = &toml.work_peers {
            config.work_peers = work_peers.clone();
        }
        if let Some(work_threads) = toml.work_threads {
            config.work_threads = work_threads;
        }
        if let Some(optimistic_scheduler_toml) = &toml.optimistic_scheduler {
            config.optimistic_scheduler = optimistic_scheduler_toml.into();
        }
        if let Some(priority_bucket_toml) = &toml.priority_bucket {
            config.priority_bucket = priority_bucket_toml.into();
        }
        if let Some(bootstrap_ascending_toml) = &toml.bootstrap_ascending {
            config.bootstrap_ascending = bootstrap_ascending_toml.into();
        }
        if let Some(bootstrap_server_toml) = &toml.bootstrap_server {
            config.bootstrap_server = bootstrap_server_toml.into();
        }
        if let Some(secondary_work_peers) = &toml.secondary_work_peers {
            config.secondary_work_peers = secondary_work_peers.clone();
        }
        if let Some(max_pruning_age_s) = toml.max_pruning_age_s {
            config.max_pruning_age_s = max_pruning_age_s;
        }
        if let Some(max_pruning_depth) = toml.max_pruning_depth {
            config.max_pruning_depth = max_pruning_depth;
        }
        if let Some(websocket_config_toml) = &toml.websocket_config {
            config.websocket_config = websocket_config_toml.into();
        }
        if let Some(ipc_config_toml) = &toml.ipc_config {
            config.ipc_config = ipc_config_toml.into();
        }
        if let Some(txn_tracking_config_toml) = &toml.txn_tracking_config {
            config.txn_tracking_config = txn_tracking_config_toml.into();
        }
        if let Some(stat_config_toml) = &toml.stat_config {
            config.stat_config = stat_config_toml.into();
        }
        if let Some(lmdb_config_toml) = &toml.lmdb_config {
            config.lmdb_config = lmdb_config_toml.into();
        }
        if let Some(backlog_scan_batch_size) = toml.backlog_scan_batch_size {
            config.backlog_scan_batch_size = backlog_scan_batch_size;
        }
        if let Some(backlog_scan_frequency) = toml.backlog_scan_frequency {
            config.backlog_scan_frequency = backlog_scan_frequency;
        }
        if let Some(vote_cache_toml) = &toml.vote_cache {
            config.vote_cache = vote_cache_toml.into();
        }
        if let Some(block_processor_toml) = &toml.block_processor {
            config.block_processor = block_processor_toml.into();
        }
        if let Some(active_elections_toml) = &toml.active_elections {
            config.active_elections = active_elections_toml.into();
        }
        if let Some(vote_processor_toml) = &toml.vote_processor {
            config.vote_processor = vote_processor_toml.into();
        }
        if let Some(request_aggregator_toml) = &toml.request_aggregator {
            config.request_aggregator = request_aggregator_toml.into();
        }
        if let Some(message_processor_toml) = &toml.message_processor {
            config.message_processor = message_processor_toml.into();
        }
        if let Some(monitor_toml) = &toml.monitor {
            config.monitor = monitor_toml.into();
        }
        if let Some(callback_address) = toml.callback_address {
            config.callback_address = callback_address;
        }

        config
    }
}

impl Default for NodeConfigToml {
    fn default() -> Self {
        let node_config = NodeConfig::default();

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
            optimistic_scheduler: Some(OptimisticSchedulerConfigToml::default()),
            priority_bucket: Some(PriorityBucketConfigToml::default()),
            bootstrap_ascending: Some(BootstrapAscendingConfigToml::default()),
            bootstrap_server: Some(BootstrapServerConfigToml::default()),
            secondary_work_peers: Some(node_config.secondary_work_peers),
            max_pruning_age_s: Some(node_config.max_pruning_age_s),
            max_pruning_depth: Some(node_config.max_pruning_depth),
            websocket_config: Some(WebsocketConfigToml::default()),
            ipc_config: Some(node_config.ipc_config.into()),
            txn_tracking_config: Some(TxnTrackingConfigToml::default()),
            stat_config: Some(StatsConfigToml::default()),
            lmdb_config: Some(LmdbConfigToml::default()),
            vote_cache: Some(VoteCacheConfigToml::default()),
            block_processor: Some(BlockProcessorConfigToml::default()),
            active_elections: Some(ActiveElectionsConfigToml::default()),
            vote_processor: Some(VoteProcessorConfigToml::default()),
            request_aggregator: Some(RequestAggregatorConfigToml::default()),
            message_processor: Some(MessageProcessorConfigToml::default()),
            monitor: Some(MonitorConfigToml::default()),
            callback_address: Some(node_config.callback_address),
            callback_port: Some(node_config.callback_port),
            callback_target: Some(node_config.callback_target),
        }
    }
}

/*impl From<NodeConfig> for NodeConfigToml {
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
            diagnostics_config: Some((&node_config.diagnostics_config).into()),
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
*/
