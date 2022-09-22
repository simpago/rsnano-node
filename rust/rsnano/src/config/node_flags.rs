use crate::{ConfirmationHeightMode, GenerateCache};

#[derive(Clone)]
pub struct NodeFlags {
    pub config_overrides: Vec<String>,
    pub rpc_config_overrides: Vec<String>,
    pub disable_add_initial_peers: bool, // For testing only
    pub disable_backup: bool,
    pub disable_lazy_bootstrap: bool,
    pub disable_legacy_bootstrap: bool,
    pub disable_wallet_bootstrap: bool,
    pub disable_bootstrap_listener: bool,
    pub disable_bootstrap_bulk_pull_server: bool,
    pub disable_bootstrap_bulk_push_client: bool,
    pub disable_ongoing_bootstrap: bool, // For testing only
    pub disable_rep_crawler: bool,
    pub disable_request_loop: bool, // For testing only
    pub disable_tcp_realtime: bool,
    pub disable_udp: bool,
    pub disable_unchecked_cleanup: bool,
    pub disable_unchecked_drop: bool,
    pub disable_providing_telemetry_metrics: bool,
    pub disable_ongoing_telemetry_requests: bool,
    pub disable_initial_telemetry_requests: bool,
    pub disable_block_processor_unchecked_deletion: bool,
    pub disable_block_processor_republishing: bool,
    pub allow_bootstrap_peers_duplicates: bool,
    pub disable_max_peers_per_ip: bool,         // For testing only
    pub disable_max_peers_per_subnetwork: bool, // For testing only
    pub force_use_write_database_queue: bool,   // For testing only
    pub disable_search_pending: bool,           // For testing only
    pub enable_pruning: bool,
    pub fast_bootstrap: bool,
    pub read_only: bool,
    pub disable_connection_cleanup: bool,
    pub confirmation_height_processor_mode: ConfirmationHeightMode,
    pub generate_cache: GenerateCache,
    pub inactive_node: bool,
    pub block_processor_batch_size: usize,
    pub block_processor_full_size: usize,
    pub block_processor_verification_size: usize,
    pub inactive_votes_cache_size: usize,
    pub vote_processor_capacity: usize,
    pub bootstrap_interval: usize, // For testing only
}

impl NodeFlags {
    pub fn new() -> Self {
        Self {
            config_overrides: Vec::new(),
            rpc_config_overrides: Vec::new(),
            disable_add_initial_peers: false,
            disable_backup: false,
            disable_lazy_bootstrap: false,
            disable_legacy_bootstrap: false,
            disable_wallet_bootstrap: false,
            disable_bootstrap_listener: false,
            disable_bootstrap_bulk_pull_server: false,
            disable_bootstrap_bulk_push_client: false,
            disable_ongoing_bootstrap: false,
            disable_rep_crawler: false,
            disable_request_loop: false,
            disable_tcp_realtime: false,
            disable_udp: true,
            disable_unchecked_cleanup: false,
            disable_unchecked_drop: true,
            disable_providing_telemetry_metrics: false,
            disable_ongoing_telemetry_requests: false,
            disable_initial_telemetry_requests: false,
            disable_block_processor_unchecked_deletion: false,
            disable_block_processor_republishing: false,
            allow_bootstrap_peers_duplicates: false,
            disable_max_peers_per_ip: false,
            disable_max_peers_per_subnetwork: false,
            force_use_write_database_queue: false,
            disable_search_pending: false,
            enable_pruning: false,
            fast_bootstrap: false,
            read_only: false,
            disable_connection_cleanup: false,
            confirmation_height_processor_mode: ConfirmationHeightMode::Automatic,
            generate_cache: GenerateCache::new(),
            inactive_node: false,
            block_processor_batch_size: 0,
            block_processor_full_size: 65536,
            block_processor_verification_size: 0,
            inactive_votes_cache_size: 1024 * 128,
            vote_processor_capacity: 144 * 1024,
            bootstrap_interval: 0,
        }
    }
}

impl Default for NodeFlags {
    fn default() -> Self {
        Self::new()
    }
}
