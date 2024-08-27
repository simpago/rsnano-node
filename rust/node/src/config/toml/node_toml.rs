use super::{
    ActiveElectionsToml, BlockProcessorToml, BootstrapAscendingToml, BootstrapServerToml,
    DiagnosticsToml, ExperimentalToml, HintedSchedulerToml, HttpcallbackToml, IpcToml, LmdbToml,
    MessageProcessorToml, MonitorToml, OptimisticSchedulerToml, PriorityBucketToml, RepCrawlerToml,
    RequestAggregatorToml, StatsToml, VoteCacheToml, VoteProcessorToml, WebsocketToml,
};
use crate::config::{FrontiersConfirmationMode, NodeConfig, Peer};
use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};

#[derive(Serialize, Deserialize)]
pub struct NodeToml {
    pub allow_local_peers: Option<bool>,
    pub background_threads: Option<u32>,
    pub backlog_scan_batch_size: Option<u32>,
    pub backlog_scan_frequency: Option<u32>,
    pub backup_before_upgrade: Option<bool>,
    pub bandwidth_limit: Option<usize>,
    pub bandwidth_limit_burst_ratio: Option<f64>,
    pub block_processor_batch_max_time: Option<i64>,
    pub bootstrap_bandwidth_burst_ratio: Option<f64>,
    pub bootstrap_bandwidth_limit: Option<usize>,
    pub bootstrap_connections: Option<u32>,
    pub bootstrap_connections_max: Option<u32>,
    pub bootstrap_fraction_numerator: Option<u32>,
    pub bootstrap_frontier_request_count: Option<u32>,
    pub bootstrap_initiator_threads: Option<u32>,
    pub bootstrap_serving_threads: Option<u32>,
    pub confirming_set_batch_time: Option<u64>,
    pub enable_voting: Option<bool>,
    pub external_address: Option<String>,
    pub external_port: Option<u16>,
    pub frontiers_confirmation: Option<String>,
    pub io_threads: Option<u32>,
    pub max_queued_requests: Option<u32>,
    pub max_unchecked_blocks: Option<u32>,
    pub max_work_generate_multiplier: Option<f64>,
    pub network_threads: Option<u32>,
    pub online_weight_minimum: Option<String>,
    pub password_fanout: Option<u32>,
    pub peering_port: Option<u16>,
    pub pow_sleep_interval: Option<i64>,
    pub preconfigured_peers: Option<Vec<String>>,
    pub preconfigured_representatives: Option<Vec<String>>,
    pub receive_minimum: Option<String>,
    pub rep_crawler_weight_minimum: Option<String>,
    pub representative_vote_weight_minimum: Option<String>,
    pub request_aggregator_threads: Option<u32>,
    pub signature_checker_threads: Option<u32>,
    pub tcp_incoming_connections_max: Option<u32>,
    pub tcp_io_timeout: Option<i64>,
    pub unchecked_cutoff_time: Option<i64>,
    pub use_memory_pools: Option<bool>,
    pub vote_generator_delay: Option<i64>,
    pub vote_generator_threshold: Option<u32>,
    pub vote_minimum: Option<String>,
    pub work_peers: Option<Vec<String>>,
    pub work_threads: Option<u32>,
    pub active_elections: Option<ActiveElectionsToml>,
    pub block_processor: Option<BlockProcessorToml>,
    pub bootstrap_ascending: Option<BootstrapAscendingToml>,
    pub bootstrap_server: Option<BootstrapServerToml>,
    pub diagnostics: Option<DiagnosticsToml>,
    pub experimental: Option<ExperimentalToml>,
    pub httpcallback: Option<HttpcallbackToml>,
    pub ipc: Option<IpcToml>,
    pub lmdb: Option<LmdbToml>,
    pub message_processor: Option<MessageProcessorToml>,
    pub monitor: Option<MonitorToml>,
    pub optimistic_scheduler: Option<OptimisticSchedulerToml>,
    pub hinted_scheduler: Option<HintedSchedulerToml>,
    pub priority_bucket: Option<PriorityBucketToml>,
    pub rep_crawler: Option<RepCrawlerToml>,
    pub request_aggregator: Option<RequestAggregatorToml>,
    pub statistics: Option<StatsToml>,
    pub vote_cache: Option<VoteCacheToml>,
    pub vote_processor: Option<VoteProcessorToml>,
    pub websocket: Option<WebsocketToml>,
}

impl NodeConfig {
    pub fn merge_toml(&mut self, toml: &NodeToml) {
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
        if let Some(block_processor_batch_max_time_ms) = toml.block_processor_batch_max_time {
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
            self.confirming_set_batch_time = Duration::from_millis(*confirming_set_batch_time);
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
        if let Some(frontiers_confirmation) = &toml.frontiers_confirmation {
            self.frontiers_confirmation = match frontiers_confirmation.as_str() {
                "always" => FrontiersConfirmationMode::Always,
                "auto" => FrontiersConfirmationMode::Automatic,
                "disabled" => FrontiersConfirmationMode::Disabled,
                _ => FrontiersConfirmationMode::Invalid,
            }
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
        if let Some(online_weight_minimum) = &toml.online_weight_minimum {
            self.online_weight_minimum =
                Amount::decode_dec(&online_weight_minimum).expect("Invalid online weight minimum");
        }
        if let Some(password_fanout) = toml.password_fanout {
            self.password_fanout = password_fanout;
        }
        if let Some(peering_port) = toml.peering_port {
            self.peering_port = Some(peering_port);
        }
        if let Some(pow_sleep_interval_ns) = toml.pow_sleep_interval {
            self.pow_sleep_interval_ns = pow_sleep_interval_ns;
        }
        if let Some(preconfigured_peers) = &toml.preconfigured_peers {
            self.preconfigured_peers = preconfigured_peers.clone();
        }
        if let Some(preconfigured_representatives) = &toml.preconfigured_representatives {
            self.preconfigured_representatives = preconfigured_representatives
                .iter()
                .map(|string| {
                    Account::decode_account(&string)
                        .expect("Invalid representative")
                        .into()
                })
                .collect();
        }
        if let Some(receive_minimum) = &toml.receive_minimum {
            self.receive_minimum =
                Amount::decode_dec(&receive_minimum).expect("Invalid receive minimum");
        }
        if let Some(rep_crawler) = &toml.rep_crawler {
            if let Some(query_timeout) = rep_crawler.query_timeout {
                self.rep_crawler_query_timeout = Duration::from_millis(query_timeout);
            }
        }
        if let Some(representative_vote_weight_minimum) = &toml.representative_vote_weight_minimum {
            self.representative_vote_weight_minimum =
                Amount::decode_dec(&representative_vote_weight_minimum)
                    .expect("Invalid representative vote weight minimum");
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
        if let Some(tcp_io_timeout_s) = toml.tcp_io_timeout {
            self.tcp_io_timeout_s = tcp_io_timeout_s;
        }
        if let Some(unchecked_cutoff_time_s) = toml.unchecked_cutoff_time {
            self.unchecked_cutoff_time_s = unchecked_cutoff_time_s;
        }
        if let Some(use_memory_pools) = toml.use_memory_pools {
            self.use_memory_pools = use_memory_pools;
        }
        if let Some(vote_generator_delay_ms) = toml.vote_generator_delay {
            self.vote_generator_delay_ms = vote_generator_delay_ms;
        }
        if let Some(vote_generator_threshold) = toml.vote_generator_threshold {
            self.vote_generator_threshold = vote_generator_threshold;
        }
        if let Some(vote_minimum) = &toml.vote_minimum {
            self.vote_minimum = Amount::decode_dec(&vote_minimum).expect("Invalid vote minimum");
        }
        if let Some(work_peers) = &toml.work_peers {
            self.work_peers = work_peers
                .iter()
                .map(|string| Peer::from_str(&string).expect("Invalid work peer"))
                .collect();
        }
        if let Some(work_threads) = toml.work_threads {
            self.work_threads = work_threads;
        }
        if let Some(optimistic_scheduler_toml) = &toml.optimistic_scheduler {
            self.optimistic_scheduler = optimistic_scheduler_toml.into();
        }
        if let Some(hinted_scheduler_toml) = &toml.hinted_scheduler {
            self.hinted_scheduler = hinted_scheduler_toml.into();
        }
        if let Some(priority_bucket_toml) = &toml.priority_bucket {
            self.priority_bucket = priority_bucket_toml.into();
        }
        if let Some(bootstrap_ascending_toml) = &toml.bootstrap_ascending {
            self.bootstrap_ascending = bootstrap_ascending_toml.into();
        }
        if let Some(bootstrap_server_toml) = &toml.bootstrap_server {
            self.bootstrap_server = bootstrap_server_toml.into();
        }
        if let Some(websocket_config_toml) = &toml.websocket {
            self.websocket_config.merge_toml(&websocket_config_toml);
        }
        if let Some(ipc_config_toml) = &toml.ipc {
            self.ipc_config.merge_toml(ipc_config_toml);
        }
        if let Some(diagnostics_config_toml) = &toml.diagnostics {
            self.diagnostics_config = diagnostics_config_toml.into();
        }
        if let Some(stat_config_toml) = &toml.statistics {
            self.stat_config = stat_config_toml.into();
        }
        if let Some(lmdb_config_toml) = &toml.lmdb {
            self.lmdb_config = lmdb_config_toml.into();
        }
        if let Some(backlog_scan_batch_size) = toml.backlog_scan_batch_size {
            self.backlog_scan_batch_size = backlog_scan_batch_size;
        }
        if let Some(backlog_scan_frequency) = toml.backlog_scan_frequency {
            self.backlog_scan_frequency = backlog_scan_frequency;
        }
        if let Some(vote_cache_toml) = &toml.vote_cache {
            self.vote_cache = vote_cache_toml.into();
        }
        if let Some(block_processor_toml) = &toml.block_processor {
            self.block_processor = block_processor_toml.into();
        }
        if let Some(active_elections_toml) = &toml.active_elections {
            self.active_elections = active_elections_toml.into();
        }
        if let Some(vote_processor_toml) = &toml.vote_processor {
            self.vote_processor.merge_toml(&vote_processor_toml);
        }
        if let Some(request_aggregator_toml) = &toml.request_aggregator {
            self.request_aggregator.merge_toml(request_aggregator_toml);
        }
        if let Some(message_processor_toml) = &toml.message_processor {
            self.message_processor.merge_toml(message_processor_toml);
        }
        if let Some(monitor_toml) = &toml.monitor {
            self.monitor = monitor_toml.into();
        }
        if let Some(rep_crawler_weight_minimum) = &toml.rep_crawler_weight_minimum {
            self.rep_crawler_weight_minimum = Amount::decode_dec(&rep_crawler_weight_minimum)
                .expect("Invalid rep crawler weight minimum");
        }
        if let Some(httpcallback) = &toml.httpcallback {
            if let Some(address) = &httpcallback.address {
                self.callback_address = address.clone();
            }
            if let Some(port) = &httpcallback.port {
                self.callback_port = port.clone();
            }
            if let Some(target) = &httpcallback.target {
                self.callback_target = target.clone();
            }
        }
    }
}

impl From<&NodeConfig> for NodeToml {
    fn from(config: &NodeConfig) -> Self {
        Self {
            allow_local_peers: Some(config.allow_local_peers),
            background_threads: Some(config.background_threads),
            backlog_scan_batch_size: Some(config.backlog_scan_batch_size),
            backlog_scan_frequency: Some(config.backlog_scan_frequency),
            backup_before_upgrade: Some(config.backup_before_upgrade),
            bandwidth_limit: Some(config.bandwidth_limit),
            bandwidth_limit_burst_ratio: Some(config.bandwidth_limit_burst_ratio),
            block_processor_batch_max_time: Some(config.block_processor_batch_max_time_ms),
            bootstrap_bandwidth_burst_ratio: Some(config.bootstrap_bandwidth_burst_ratio),
            bootstrap_bandwidth_limit: Some(config.bootstrap_bandwidth_limit),
            bootstrap_connections: Some(config.bootstrap_connections),
            bootstrap_connections_max: Some(config.bootstrap_connections_max),
            bootstrap_fraction_numerator: Some(config.bootstrap_fraction_numerator),
            bootstrap_frontier_request_count: Some(config.bootstrap_frontier_request_count),
            bootstrap_initiator_threads: Some(config.bootstrap_initiator_threads),
            bootstrap_serving_threads: Some(config.bootstrap_serving_threads),
            confirming_set_batch_time: Some(config.confirming_set_batch_time.as_millis() as u64),
            enable_voting: Some(config.enable_voting),
            external_address: Some(config.external_address.clone()),
            external_port: Some(config.external_port),
            frontiers_confirmation: Some(match config.frontiers_confirmation {
                FrontiersConfirmationMode::Always => "always".to_string(),
                FrontiersConfirmationMode::Automatic => "auto".to_string(),
                FrontiersConfirmationMode::Disabled => "disabled".to_string(),
                FrontiersConfirmationMode::Invalid => "invalid".to_string(),
            }),
            io_threads: Some(config.io_threads),
            max_queued_requests: Some(config.max_queued_requests),
            max_unchecked_blocks: Some(config.max_unchecked_blocks),
            max_work_generate_multiplier: Some(config.max_work_generate_multiplier),
            network_threads: Some(config.network_threads),
            online_weight_minimum: Some(config.online_weight_minimum.to_string_dec()),
            password_fanout: Some(config.password_fanout),
            peering_port: config.peering_port,
            pow_sleep_interval: Some(config.pow_sleep_interval_ns),
            preconfigured_peers: Some(config.preconfigured_peers.clone()),
            preconfigured_representatives: Some(
                config
                    .preconfigured_representatives
                    .iter()
                    .map(|pk| Account::from(pk).encode_account())
                    .collect(),
            ),
            receive_minimum: Some(config.receive_minimum.to_string_dec()),
            rep_crawler_weight_minimum: Some(config.rep_crawler_weight_minimum.to_string_dec()),
            representative_vote_weight_minimum: Some(
                config.representative_vote_weight_minimum.to_string_dec(),
            ),
            request_aggregator_threads: Some(config.request_aggregator_threads),
            signature_checker_threads: Some(config.signature_checker_threads),
            tcp_incoming_connections_max: Some(config.tcp_incoming_connections_max),
            tcp_io_timeout: Some(config.tcp_io_timeout_s),
            unchecked_cutoff_time: Some(config.unchecked_cutoff_time_s),
            use_memory_pools: Some(config.use_memory_pools),
            vote_generator_delay: Some(config.vote_generator_delay_ms),
            vote_generator_threshold: Some(config.vote_generator_threshold),
            vote_minimum: Some(config.vote_minimum.to_string_dec()),
            work_peers: Some(
                config
                    .work_peers
                    .iter()
                    .map(|peer| peer.to_string())
                    .collect(),
            ),
            work_threads: Some(config.work_threads),
            optimistic_scheduler: Some((&config.optimistic_scheduler).into()),
            hinted_scheduler: Some((&config.hinted_scheduler).into()),
            priority_bucket: Some((&config.priority_bucket).into()),
            bootstrap_ascending: Some((&config.bootstrap_ascending).into()),
            bootstrap_server: Some((&config.bootstrap_server).into()),
            websocket: Some((&config.websocket_config).into()),
            ipc: Some((&config.ipc_config).into()),
            diagnostics: Some((&config.diagnostics_config).into()),
            statistics: Some((&config.stat_config).into()),
            lmdb: Some((&config.lmdb_config).into()),
            vote_cache: Some((&config.vote_cache).into()),
            block_processor: Some((&config.block_processor).into()),
            active_elections: Some((&config.active_elections).into()),
            vote_processor: Some((&config.vote_processor).into()),
            request_aggregator: Some((&config.request_aggregator).into()),
            message_processor: Some((&config.message_processor).into()),
            monitor: Some((&config.monitor).into()),
            httpcallback: Some(config.into()),
            rep_crawler: Some(config.into()),
            experimental: Some(config.into()),
        }
    }
}
