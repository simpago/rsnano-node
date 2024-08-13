use super::{
    bootstrap_config::BootstrapAscendingConfigDto,
    fill_txn_tracking_config_dto, fill_websocket_config_dto,
    lmdb_config::{fill_lmdb_config_dto, LmdbConfigDto},
    TxnTrackingConfigDto,
};
use crate::{
    bootstrap::BootstrapServerConfigDto,
    consensus::{
        ActiveElectionsConfigDto, RequestAggregatorConfigDto, VoteCacheConfigDto,
        VoteProcessorConfigDto,
    },
    fill_ipc_config_dto, fill_stat_config_dto, BlockProcessorConfigDto, HintedSchedulerConfigDto,
    IpcConfigDto, NetworkParamsDto, OptimisticSchedulerConfigDto, StatConfigDto,
    WebsocketConfigDto,
};
use num::FromPrimitive;
use rsnano_core::{utils::get_cpu_count, Account, Amount};
use rsnano_node::{
    block_processing::LocalBlockBroadcasterConfig,
    cementation::ConfirmingSetConfig,
    config::{MonitorConfig, NodeConfig, Peer},
    consensus::PriorityBucketConfig,
    transport::{MessageProcessorConfig, TcpConfig},
    NetworkParams,
};
use std::{
    convert::{TryFrom, TryInto},
    ffi::c_void,
    time::Duration,
};

#[repr(C)]
pub struct NodeConfigDto {
    pub peering_port: u16,
    pub optimistic_scheduler: OptimisticSchedulerConfigDto,
    pub hinted_scheduler: HintedSchedulerConfigDto,
    pub priority_bucket: PriorityBucketConfigDto,
    pub peering_port_defined: bool,
    pub bootstrap_fraction_numerator: u32,
    pub receive_minimum: [u8; 16],
    pub online_weight_minimum: [u8; 16],
    pub representative_vote_weight_minimum: [u8; 16],
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
    pub vote_minimum: [u8; 16],
    pub vote_generator_delay_ms: i64,
    pub vote_generator_threshold: u32,
    pub unchecked_cutoff_time_s: i64,
    pub tcp_io_timeout_s: i64,
    pub pow_sleep_interval_ns: i64,
    pub external_address: [u8; 128],
    pub external_address_len: usize,
    pub external_port: u16,
    pub tcp_incoming_connections_max: u32,
    pub use_memory_pools: bool,
    pub bandwidth_limit: usize,
    pub bandwidth_limit_burst_ratio: f64,
    pub bootstrap_ascending: BootstrapAscendingConfigDto,
    pub bootstrap_server: BootstrapServerConfigDto,
    pub bootstrap_bandwidth_limit: usize,
    pub bootstrap_bandwidth_burst_ratio: f64,
    pub confirming_set_batch_time_ms: i64,
    pub backup_before_upgrade: bool,
    pub max_work_generate_multiplier: f64,
    pub frontiers_confirmation: u8,
    pub max_queued_requests: u32,
    pub request_aggregator_threads: u32,
    pub max_unchecked_blocks: u32,
    pub rep_crawler_weight_minimum: [u8; 16],
    pub work_peers: [PeerDto; 5],
    pub work_peers_count: usize,
    pub secondary_work_peers: [PeerDto; 50],
    pub secondary_work_peers_count: usize,
    pub preconfigured_peers: [PeerDto; 50],
    pub preconfigured_peers_count: usize,
    pub preconfigured_representatives: [[u8; 32]; 10],
    pub preconfigured_representatives_count: usize,
    pub max_pruning_age_s: i64,
    pub max_pruning_depth: u64,
    pub callback_address: [u8; 128],
    pub callback_address_len: usize,
    pub callback_port: u16,
    pub callback_target: [u8; 128],
    pub callback_target_len: usize,
    pub websocket_config: WebsocketConfigDto,
    pub ipc_config: IpcConfigDto,
    pub diagnostics_config: TxnTrackingConfigDto,
    pub stat_config: StatConfigDto,
    pub lmdb_config: LmdbConfigDto,
    pub backlog_scan_batch_size: u32,
    pub backlog_scan_frequency: u32,
    pub vote_cache: VoteCacheConfigDto,
    pub rep_crawler_query_timeout_ms: i64,
    pub block_processor: BlockProcessorConfigDto,
    pub active_elections: ActiveElectionsConfigDto,
    pub vote_processor: VoteProcessorConfigDto,
    pub tcp: TcpConfigDto,
    pub request_aggregator: RequestAggregatorConfigDto,
    pub message_processor: MessageProcessorConfigDto,
    pub priority_scheduler_enabled: bool,
    pub local_block_broadcaster: LocalBlockBroadcasterConfigDto,
    pub confirming_set: ConfirmingSetConfigDto,
    pub monitor: MonitorConfigDto,
}

#[repr(C)]
pub struct TcpConfigDto {
    pub max_inbound_connections: usize,
    pub max_outbound_connections: usize,
    pub max_attempts: usize,
    pub max_attempts_per_ip: usize,
    pub connect_timeout_s: u64,
}

impl From<&TcpConfig> for TcpConfigDto {
    fn from(value: &TcpConfig) -> Self {
        Self {
            max_inbound_connections: value.max_inbound_connections,
            max_outbound_connections: value.max_outbound_connections,
            max_attempts: value.max_attempts,
            max_attempts_per_ip: value.max_attempts_per_ip,
            connect_timeout_s: value.connect_timeout.as_secs(),
        }
    }
}

impl From<&TcpConfigDto> for TcpConfig {
    fn from(value: &TcpConfigDto) -> Self {
        Self {
            max_inbound_connections: value.max_inbound_connections,
            max_outbound_connections: value.max_outbound_connections,
            max_attempts: value.max_attempts,
            max_attempts_per_ip: value.max_attempts_per_ip,
            connect_timeout: Duration::from_secs(value.connect_timeout_s),
        }
    }
}

#[repr(C)]
pub struct PeerDto {
    pub address: [u8; 128],
    pub address_len: usize,
    pub port: u16,
}

#[repr(C)]
pub struct MonitorConfigDto {
    pub enabled: bool,
    pub interval_s: u64,
}

impl From<&MonitorConfig> for MonitorConfigDto {
    fn from(value: &MonitorConfig) -> Self {
        Self {
            enabled: value.enabled,
            interval_s: value.interval.as_secs(),
        }
    }
}

impl From<&MonitorConfigDto> for MonitorConfig {
    fn from(value: &MonitorConfigDto) -> Self {
        Self {
            enabled: value.enabled,
            interval: Duration::from_secs(value.interval_s),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_config_create(
    dto: *mut NodeConfigDto,
    peering_port: u16,
    peering_port_defined: bool,
    network_params: &NetworkParamsDto,
) -> i32 {
    let network_params = match NetworkParams::try_from(network_params) {
        Ok(n) => n,
        Err(_) => return -1,
    };
    let peering_port = if peering_port_defined {
        Some(peering_port)
    } else {
        None
    };
    let cfg = NodeConfig::new(peering_port, &network_params, get_cpu_count());
    let dto = &mut (*dto);
    fill_node_config_dto(dto, &cfg);
    0
}

pub fn fill_node_config_dto(dto: &mut NodeConfigDto, cfg: &NodeConfig) {
    dto.peering_port = cfg.peering_port.unwrap_or_default();
    dto.optimistic_scheduler = (&cfg.optimistic_scheduler).into();
    dto.hinted_scheduler = (&cfg.hinted_scheduler).into();
    dto.priority_bucket = (&cfg.priority_bucket).into();
    dto.peering_port_defined = cfg.peering_port.is_some();
    dto.bootstrap_fraction_numerator = cfg.bootstrap_fraction_numerator;
    dto.receive_minimum = cfg.receive_minimum.to_be_bytes();
    dto.online_weight_minimum = cfg.online_weight_minimum.to_be_bytes();
    dto.representative_vote_weight_minimum = cfg.representative_vote_weight_minimum.to_be_bytes();
    dto.password_fanout = cfg.password_fanout;
    dto.io_threads = cfg.io_threads;
    dto.network_threads = cfg.network_threads;
    dto.work_threads = cfg.work_threads;
    dto.background_threads = cfg.background_threads;
    dto.signature_checker_threads = cfg.signature_checker_threads;
    dto.enable_voting = cfg.enable_voting;
    dto.bootstrap_connections = cfg.bootstrap_connections;
    dto.bootstrap_connections_max = cfg.bootstrap_connections_max;
    dto.bootstrap_initiator_threads = cfg.bootstrap_initiator_threads;
    dto.bootstrap_serving_threads = cfg.bootstrap_serving_threads;
    dto.bootstrap_frontier_request_count = cfg.bootstrap_frontier_request_count;
    dto.block_processor_batch_max_time_ms = cfg.block_processor_batch_max_time_ms;
    dto.allow_local_peers = cfg.allow_local_peers;
    dto.vote_minimum = cfg.vote_minimum.to_be_bytes();
    dto.vote_generator_delay_ms = cfg.vote_generator_delay_ms;
    dto.vote_generator_threshold = cfg.vote_generator_threshold;
    dto.unchecked_cutoff_time_s = cfg.unchecked_cutoff_time_s;
    dto.tcp_io_timeout_s = cfg.tcp_io_timeout_s;
    dto.pow_sleep_interval_ns = cfg.pow_sleep_interval_ns;
    let bytes: &[u8] = cfg.external_address.as_bytes();
    dto.external_address[..bytes.len()].copy_from_slice(bytes);
    dto.external_address_len = bytes.len();
    dto.external_port = cfg.external_port;
    dto.tcp_incoming_connections_max = cfg.tcp_incoming_connections_max;
    dto.use_memory_pools = cfg.use_memory_pools;

    dto.bandwidth_limit = cfg.bandwidth_limit;
    dto.bandwidth_limit_burst_ratio = cfg.bandwidth_limit_burst_ratio;
    dto.bootstrap_bandwidth_limit = cfg.bootstrap_bandwidth_limit;
    dto.bootstrap_bandwidth_burst_ratio = cfg.bootstrap_bandwidth_burst_ratio;
    dto.bootstrap_ascending = (&cfg.bootstrap_ascending).into();
    dto.bootstrap_server = (&cfg.bootstrap_server).into();
    dto.confirming_set_batch_time_ms = cfg.confirming_set_batch_time.as_millis() as i64;
    dto.backup_before_upgrade = cfg.backup_before_upgrade;
    dto.max_work_generate_multiplier = cfg.max_work_generate_multiplier;
    dto.frontiers_confirmation = cfg.frontiers_confirmation as u8;
    dto.max_queued_requests = cfg.max_queued_requests;
    dto.request_aggregator_threads = cfg.request_aggregator_threads;
    dto.max_unchecked_blocks = cfg.max_unchecked_blocks;
    dto.rep_crawler_weight_minimum = cfg.rep_crawler_weight_minimum.to_be_bytes();
    if cfg.work_peers.len() > dto.work_peers.len() {
        panic!(
            "RsNano does currently not support more than {} work peers",
            dto.preconfigured_representatives.len()
        );
    }
    for (i, peer) in cfg.work_peers.iter().enumerate() {
        let bytes = peer.address.as_bytes();
        dto.work_peers[i].address[..bytes.len()].copy_from_slice(bytes);
        dto.work_peers[i].address_len = bytes.len();
        dto.work_peers[i].port = peer.port;
    }
    dto.work_peers_count = cfg.work_peers.len();
    if cfg.secondary_work_peers.len() > dto.secondary_work_peers.len() {
        panic!(
            "RsNano does currently not support more than {} secondary work peers",
            dto.secondary_work_peers.len()
        );
    }
    for (i, peer) in cfg.secondary_work_peers.iter().enumerate() {
        let bytes = peer.address.as_bytes();
        dto.secondary_work_peers[i].address[..bytes.len()].copy_from_slice(bytes);
        dto.secondary_work_peers[i].address_len = bytes.len();
        dto.secondary_work_peers[i].port = peer.port;
    }
    dto.secondary_work_peers_count = cfg.secondary_work_peers.len();

    if cfg.preconfigured_peers.len() > dto.preconfigured_peers.len() {
        panic!(
            "RsNano does currently not support more than {} preconfigured peers",
            dto.preconfigured_peers.len()
        );
    }
    for (i, peer) in cfg.preconfigured_peers.iter().enumerate() {
        let bytes = peer.as_bytes();
        dto.preconfigured_peers[i].address[..bytes.len()].copy_from_slice(bytes);
        dto.preconfigured_peers[i].address_len = bytes.len();
    }
    dto.preconfigured_peers_count = cfg.preconfigured_peers.len();
    if cfg.preconfigured_representatives.len() > dto.preconfigured_representatives.len() {
        panic!(
            "RsNano does currently not support more than {} preconfigured representatives",
            dto.preconfigured_representatives.len()
        );
    }
    for (i, rep) in cfg.preconfigured_representatives.iter().enumerate() {
        dto.preconfigured_representatives[i] = *rep.as_bytes();
    }
    dto.preconfigured_representatives_count = cfg.preconfigured_representatives.len();
    dto.max_pruning_age_s = cfg.max_pruning_age_s;
    dto.max_pruning_depth = cfg.max_pruning_depth;
    let bytes = cfg.callback_address.as_bytes();
    dto.callback_address[..bytes.len()].copy_from_slice(bytes);
    dto.callback_address_len = bytes.len();
    dto.callback_port = cfg.callback_port;
    let bytes = cfg.callback_target.as_bytes();
    dto.callback_target[..bytes.len()].copy_from_slice(bytes);
    dto.callback_target_len = bytes.len();
    fill_websocket_config_dto(&mut dto.websocket_config, &cfg.websocket_config);
    fill_ipc_config_dto(&mut dto.ipc_config, &cfg.ipc_config);
    fill_txn_tracking_config_dto(
        &mut dto.diagnostics_config,
        &cfg.diagnostics_config.txn_tracking,
    );
    fill_stat_config_dto(&mut dto.stat_config, &cfg.stat_config);
    fill_lmdb_config_dto(&mut dto.lmdb_config, &cfg.lmdb_config);
    dto.backlog_scan_frequency = cfg.backlog_scan_frequency;
    dto.backlog_scan_batch_size = cfg.backlog_scan_batch_size;
    dto.vote_cache = (&cfg.vote_cache).into();
    dto.rep_crawler_query_timeout_ms = cfg.rep_crawler_query_timeout.as_millis() as i64;
    dto.block_processor = (&cfg.block_processor).into();
    dto.active_elections = (&cfg.active_elections).into();
    dto.vote_processor = (&cfg.vote_processor).into();
    dto.tcp = (&cfg.tcp).into();
    dto.request_aggregator = (&cfg.request_aggregator).into();
    dto.message_processor = (&cfg.message_processor).into();
    dto.priority_scheduler_enabled = cfg.priority_scheduler_enabled;
    dto.local_block_broadcaster = (&cfg.local_block_broadcaster).into();
    dto.confirming_set = (&cfg.confirming_set).into();
    dto.monitor = (&cfg.monitor).into();
}

#[no_mangle]
pub extern "C" fn rsn_node_config_serialize_toml(dto: &NodeConfigDto, toml: *mut c_void) -> i32 {
    let cfg = match NodeConfig::try_from(dto) {
        Ok(c) => c,
        Err(_) => return -1,
    };
    0
}

impl From<&PeerDto> for Peer {
    fn from(value: &PeerDto) -> Self {
        let address = String::from_utf8_lossy(&value.address[..value.address_len]).to_string();
        Peer::new(address, value.port)
    }
}

impl TryFrom<&NodeConfigDto> for NodeConfig {
    type Error = anyhow::Error;

    fn try_from(value: &NodeConfigDto) -> Result<Self, Self::Error> {
        let mut work_peers = Vec::with_capacity(value.work_peers_count);
        for i in 0..value.work_peers_count {
            work_peers.push((&value.work_peers[i]).into());
        }

        let mut secondary_work_peers = Vec::with_capacity(value.secondary_work_peers_count);
        for i in 0..value.secondary_work_peers_count {
            secondary_work_peers.push((&value.secondary_work_peers[i]).into());
        }

        let mut preconfigured_peers = Vec::with_capacity(value.preconfigured_peers_count);
        for i in 0..value.preconfigured_peers_count {
            preconfigured_peers.push(Peer::from(&value.preconfigured_peers[i]).address);
        }

        let mut preconfigured_representatives = Vec::new();
        for i in 0..value.preconfigured_representatives_count {
            preconfigured_representatives
                .push(Account::from_bytes(value.preconfigured_representatives[i]));
        }

        let cfg = NodeConfig {
            peering_port: if value.peering_port_defined {
                Some(value.peering_port)
            } else {
                None
            },
            optimistic_scheduler: (&value.optimistic_scheduler).into(),
            hinted_scheduler: (&value.hinted_scheduler).into(),
            priority_bucket: (&value.priority_bucket).into(),
            bootstrap_fraction_numerator: value.bootstrap_fraction_numerator,
            receive_minimum: Amount::from_be_bytes(value.receive_minimum),
            online_weight_minimum: Amount::from_be_bytes(value.online_weight_minimum),
            representative_vote_weight_minimum: Amount::from_be_bytes(
                value.representative_vote_weight_minimum,
            ),
            password_fanout: value.password_fanout,
            io_threads: value.io_threads,
            network_threads: value.network_threads,
            work_threads: value.work_threads,
            background_threads: value.background_threads,
            signature_checker_threads: value.signature_checker_threads,
            enable_voting: value.enable_voting,
            bootstrap_connections: value.bootstrap_connections,
            bootstrap_connections_max: value.bootstrap_connections_max,
            bootstrap_initiator_threads: value.bootstrap_initiator_threads,
            bootstrap_serving_threads: value.bootstrap_serving_threads,
            bootstrap_frontier_request_count: value.bootstrap_frontier_request_count,
            block_processor_batch_max_time_ms: value.block_processor_batch_max_time_ms,
            allow_local_peers: value.allow_local_peers,
            vote_minimum: Amount::from_be_bytes(value.vote_minimum),
            vote_generator_delay_ms: value.vote_generator_delay_ms,
            vote_generator_threshold: value.vote_generator_threshold,
            unchecked_cutoff_time_s: value.unchecked_cutoff_time_s,
            tcp_io_timeout_s: value.tcp_io_timeout_s,
            pow_sleep_interval_ns: value.pow_sleep_interval_ns,
            external_address: String::from_utf8_lossy(
                &value.external_address[..value.external_address_len],
            )
            .to_string(),
            external_port: value.external_port,
            tcp_incoming_connections_max: value.tcp_incoming_connections_max,
            use_memory_pools: value.use_memory_pools,
            bandwidth_limit: value.bandwidth_limit,
            bandwidth_limit_burst_ratio: value.bandwidth_limit_burst_ratio,
            bootstrap_bandwidth_limit: value.bootstrap_bandwidth_limit,
            bootstrap_bandwidth_burst_ratio: value.bootstrap_bandwidth_burst_ratio,
            bootstrap_ascending: (&value.bootstrap_ascending).into(),
            bootstrap_server: (&value.bootstrap_server).into(),
            confirming_set_batch_time: Duration::from_millis(
                value.confirming_set_batch_time_ms as u64,
            ),
            backup_before_upgrade: value.backup_before_upgrade,
            max_work_generate_multiplier: value.max_work_generate_multiplier,
            frontiers_confirmation: FromPrimitive::from_u8(value.frontiers_confirmation)
                .ok_or_else(|| anyhow!("invalid frontiers confirmation mode"))?,
            max_queued_requests: value.max_queued_requests,
            request_aggregator_threads: value.request_aggregator_threads,
            max_unchecked_blocks: value.max_unchecked_blocks,
            rep_crawler_weight_minimum: Amount::from_be_bytes(value.rep_crawler_weight_minimum),
            work_peers,
            secondary_work_peers,
            preconfigured_peers,
            preconfigured_representatives,
            max_pruning_age_s: value.max_pruning_age_s,
            max_pruning_depth: value.max_pruning_depth,
            callback_address: String::from_utf8_lossy(
                &value.callback_address[..value.callback_address_len],
            )
            .to_string(),
            callback_target: String::from_utf8_lossy(
                &value.callback_target[..value.callback_target_len],
            )
            .to_string(),
            callback_port: value.callback_port,
            websocket_config: (&value.websocket_config).into(),
            ipc_config: (&value.ipc_config).try_into()?,
            diagnostics_config: (&value.diagnostics_config).into(),
            stat_config: (&value.stat_config).into(),
            lmdb_config: (&value.lmdb_config).into(),
            backlog_scan_batch_size: value.backlog_scan_batch_size,
            backlog_scan_frequency: value.backlog_scan_frequency,
            vote_cache: (&value.vote_cache).into(),
            rep_crawler_query_timeout: Duration::from_millis(
                value.rep_crawler_query_timeout_ms as u64,
            ),
            block_processor: (&value.block_processor).into(),
            active_elections: (&value.active_elections).into(),
            vote_processor: (&value.vote_processor).into(),
            tcp: (&value.tcp).into(),
            request_aggregator: (&value.request_aggregator).into(),
            message_processor: (&value.message_processor).into(),
            priority_scheduler_enabled: value.priority_scheduler_enabled,
            local_block_broadcaster: (&value.local_block_broadcaster).into(),
            confirming_set: (&value.confirming_set).into(),
            monitor: (&value.monitor).into(),
        };

        Ok(cfg)
    }
}

#[repr(C)]
pub struct MessageProcessorConfigDto {
    pub threads: usize,
    pub max_queue: usize,
}

impl From<&MessageProcessorConfigDto> for MessageProcessorConfig {
    fn from(value: &MessageProcessorConfigDto) -> Self {
        Self {
            threads: value.threads,
            max_queue: value.max_queue,
        }
    }
}

impl From<&MessageProcessorConfig> for MessageProcessorConfigDto {
    fn from(value: &MessageProcessorConfig) -> Self {
        Self {
            threads: value.threads,
            max_queue: value.max_queue,
        }
    }
}

#[repr(C)]
pub struct LocalBlockBroadcasterConfigDto {
    pub max_size: usize,
    pub rebroadcast_interval_s: u64,
    pub max_rebroadcast_interval_s: u64,
    pub broadcast_rate_limit: usize,
    pub broadcast_rate_burst_ratio: f64,
    pub cleanup_interval_s: u64,
}

impl From<&LocalBlockBroadcasterConfig> for LocalBlockBroadcasterConfigDto {
    fn from(value: &LocalBlockBroadcasterConfig) -> Self {
        Self {
            max_size: value.max_size,
            rebroadcast_interval_s: value.rebroadcast_interval.as_secs() as u64,
            max_rebroadcast_interval_s: value.max_rebroadcast_interval.as_secs() as u64,
            broadcast_rate_limit: value.broadcast_rate_limit,
            broadcast_rate_burst_ratio: value.broadcast_rate_burst_ratio,
            cleanup_interval_s: value.cleanup_interval.as_secs() as u64,
        }
    }
}

impl From<&LocalBlockBroadcasterConfigDto> for LocalBlockBroadcasterConfig {
    fn from(value: &LocalBlockBroadcasterConfigDto) -> Self {
        Self {
            max_size: value.max_size,
            rebroadcast_interval: Duration::from_secs(value.rebroadcast_interval_s),
            max_rebroadcast_interval: Duration::from_secs(value.max_rebroadcast_interval_s),
            broadcast_rate_limit: value.broadcast_rate_limit,
            broadcast_rate_burst_ratio: value.broadcast_rate_burst_ratio,
            cleanup_interval: Duration::from_secs(value.cleanup_interval_s),
        }
    }
}

#[repr(C)]
pub struct ConfirmingSetConfigDto {
    pub max_blocks: usize,
    pub max_queued_notifications: usize,
}

impl From<&ConfirmingSetConfigDto> for ConfirmingSetConfig {
    fn from(value: &ConfirmingSetConfigDto) -> Self {
        Self {
            max_blocks: value.max_blocks,
            max_queued_notifications: value.max_queued_notifications,
        }
    }
}

impl From<&ConfirmingSetConfig> for ConfirmingSetConfigDto {
    fn from(value: &ConfirmingSetConfig) -> Self {
        Self {
            max_blocks: value.max_blocks,
            max_queued_notifications: value.max_queued_notifications,
        }
    }
}

#[repr(C)]
pub struct PriorityBucketConfigDto {
    pub max_blocks: usize,
    pub reserved_elections: usize,
    pub max_elections: usize,
}

impl From<&PriorityBucketConfigDto> for PriorityBucketConfig {
    fn from(value: &PriorityBucketConfigDto) -> Self {
        Self {
            max_blocks: value.max_blocks,
            reserved_elections: value.reserved_elections,
            max_elections: value.max_elections,
        }
    }
}

impl From<&PriorityBucketConfig> for PriorityBucketConfigDto {
    fn from(value: &PriorityBucketConfig) -> Self {
        Self {
            max_blocks: value.max_blocks,
            reserved_elections: value.reserved_elections,
            max_elections: value.max_elections,
        }
    }
}
