mod active_elections_config_toml;
mod block_processor_config_toml;
mod bootstrap_ascending_config_toml;
mod bootstrap_server_config_toml;
mod ipc_config_toml;
mod lmdb_config_toml;
mod message_processor_config_toml;
mod monitor_config_toml;
mod node_config_toml;
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
use rsnano_core::utils::TomlWriter;
pub use stats_config_toml::*;
pub use txn_tracking_config_toml::*;
pub use vote_cache_config_toml::*;
pub use vote_processor_config_toml::*;
pub use websocket_config_toml::*;

use crate::config::{Miliseconds, NodeConfig};
use crate::node::{FrontiersConfirmationMode, Peer};
use anyhow::Result;
use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct NodeConfigToml {
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
    pub(crate) optimistic_scheduler: Option<OptimisticSchedulerConfigToml>,
    pub(crate) priority_bucket: Option<PriorityBucketConfigToml>,
    pub(crate) bootstrap_ascending: Option<BootstrapAscendingConfigToml>,
    pub(crate) bootstrap_server: Option<BootstrapServerConfigToml>,
    pub(crate) secondary_work_peers: Option<Vec<Peer>>,
    pub(crate) max_pruning_age_s: Option<i64>,
    pub(crate) max_pruning_depth: Option<u64>,
    pub(crate) websocket_config: Option<WebsocketConfigToml>,
    pub(crate) ipc_config: Option<TomlIpcConfig>,
    pub(crate) diagnostics_config: Option<DiagnosticsConfigToml>,
    pub(crate) stat_config: Option<StatsConfigToml>,
    pub(crate) lmdb_config: Option<TomlLmdbConfig>,
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

impl NodeConfigToml {
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
        if let Some(diagnostics_config_toml) = &toml.diagnostics_config {
            config.diagnostics_config = diagnostics_config_toml.into();
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
