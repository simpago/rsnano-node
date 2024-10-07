use super::{FrontiersConfirmationMode, GlobalConfig};
use crate::{
    block_processing::{BacklogPopulationConfig, BlockProcessorConfig},
    bootstrap::BootstrapInitiatorConfig,
};
use rsnano_network::{bandwidth_limiter::OutboundBandwidthLimiterConfig, NetworkConfig};
use std::time::Duration;

impl From<&GlobalConfig> for BlockProcessorConfig {
    fn from(value: &GlobalConfig) -> Self {
        let config = &value.node_config.block_processor;
        Self {
            max_peer_queue: config.max_peer_queue,
            priority_local: config.priority_local,
            priority_bootstrap: config.priority_bootstrap,
            priority_live: config.priority_live,
            max_system_queue: config.max_system_queue,
            batch_max_time: Duration::from_millis(
                value.node_config.block_processor_batch_max_time_ms as u64,
            ),
            full_size: value.flags.block_processor_full_size,
            batch_size: value.flags.block_processor_batch_size,
            work_thresholds: value.network_params.work.clone(),
        }
    }
}

impl From<&GlobalConfig> for BootstrapInitiatorConfig {
    fn from(value: &GlobalConfig) -> Self {
        Self {
            bootstrap_connections: value.node_config.bootstrap_connections,
            bootstrap_connections_max: value.node_config.bootstrap_connections_max,
            tcp_io_timeout: Duration::from_secs(value.node_config.tcp_io_timeout_s as u64),
            silent_connection_tolerance_time: Duration::from_secs(
                value
                    .network_params
                    .network
                    .silent_connection_tolerance_time_s as u64,
            ),
            allow_bootstrap_peers_duplicates: value.flags.allow_bootstrap_peers_duplicates,
            disable_legacy_bootstrap: value.flags.disable_legacy_bootstrap,
            idle_timeout: value.network_params.network.idle_timeout,
            lazy_max_pull_blocks: value.network_params.bootstrap.lazy_max_pull_blocks,
            work_thresholds: value.network_params.work.clone(),
            lazy_retry_limit: value.network_params.bootstrap.lazy_retry_limit,
            protocol: value.network_params.network.protocol_info(),
            frontier_request_count: value.node_config.bootstrap_frontier_request_count,
            frontier_retry_limit: value.network_params.bootstrap.frontier_retry_limit,
            disable_bulk_push_client: value.flags.disable_bootstrap_bulk_push_client,
            bootstrap_initiator_threads: value.node_config.bootstrap_initiator_threads,
            receive_minimum: *value.node_config.receive_minimum.lock().unwrap(),
        }
    }
}

impl From<&GlobalConfig> for BacklogPopulationConfig {
    fn from(value: &GlobalConfig) -> Self {
        Self {
            enabled: value.node_config.frontiers_confirmation
                != FrontiersConfirmationMode::Disabled,
            batch_size: value.node_config.backlog_scan_batch_size,
            frequency: value.node_config.backlog_scan_frequency,
        }
    }
}

impl From<&GlobalConfig> for NetworkConfig {
    fn from(value: &GlobalConfig) -> Self {
        Self {
            max_inbound_connections: value.node_config.tcp.max_inbound_connections,
            max_outbound_connections: value.node_config.tcp.max_outbound_connections,
            max_peers_per_ip: value.network_params.network.max_peers_per_ip,
            max_peers_per_subnetwork: value.network_params.network.max_peers_per_subnetwork,
            max_attempts_per_ip: value.node_config.tcp.max_attempts_per_ip,
            allow_local_peers: value.node_config.allow_local_peers,
            disable_max_peers_per_ip: value.flags.disable_max_peers_per_ip,
            disable_max_peers_per_subnetwork: value.flags.disable_max_peers_per_subnetwork,
            disable_network: value.flags.disable_tcp_realtime,
            min_protocol_version: value.network_params.network.protocol_info().version_min,
            listening_port: value.node_config.peering_port.unwrap_or(0),
        }
    }
}

impl From<&GlobalConfig> for OutboundBandwidthLimiterConfig {
    fn from(value: &GlobalConfig) -> Self {
        Self {
            standard_limit: value.node_config.bandwidth_limit,
            standard_burst_ratio: value.node_config.bandwidth_limit_burst_ratio,
            bootstrap_limit: value.node_config.bootstrap_bandwidth_limit,
            bootstrap_burst_ratio: value.node_config.bootstrap_bandwidth_burst_ratio,
        }
    }
}
