use super::BlockProcessorConfig;
use rsnano_ledger::GenerateCacheFlags;

#[derive(Clone)]
pub struct NodeFlags {
    pub config_overrides: Vec<String>,
    pub rpc_config_overrides: Vec<String>,
    pub disable_activate_successors: bool,
    pub disable_backup: bool,
    pub disable_lazy_bootstrap: bool,
    pub disable_legacy_bootstrap: bool,
    pub disable_wallet_bootstrap: bool,
    pub disable_bootstrap_listener: bool,
    pub disable_bootstrap_bulk_pull_server: bool,
    pub disable_bootstrap_bulk_push_client: bool,
    pub disable_ongoing_bootstrap: bool, // For testing only
    pub disable_ascending_bootstrap: bool,
    pub disable_rep_crawler: bool,
    pub disable_request_loop: bool, // For testing only
    pub disable_tcp_realtime: bool,
    pub disable_providing_telemetry_metrics: bool,
    pub disable_ongoing_telemetry_requests: bool,
    pub disable_block_processor_unchecked_deletion: bool,
    pub disable_block_processor_republishing: bool,
    pub allow_bootstrap_peers_duplicates: bool,
    pub disable_max_peers_per_ip: bool,         // For testing only
    pub disable_max_peers_per_subnetwork: bool, // For testing only
    pub force_use_write_queue: bool,            // For testing only
    pub disable_search_pending: bool,           // For testing only
    pub enable_pruning: bool,
    pub fast_bootstrap: bool,
    pub read_only: bool,
    pub disable_connection_cleanup: bool,
    pub generate_cache: GenerateCacheFlags,
    pub inactive_node: bool,
    pub block_processor_batch_size: usize,
    pub block_processor_full_size: usize,
    pub block_processor_verification_size: usize,
    pub vote_processor_capacity: usize,
    pub bootstrap_interval: usize, // For testing only
}

impl NodeFlags {
    pub fn new() -> Self {
        Self {
            config_overrides: Vec::new(),
            rpc_config_overrides: Vec::new(),
            disable_activate_successors: false,
            disable_backup: false,
            disable_lazy_bootstrap: false,
            disable_legacy_bootstrap: false,
            disable_wallet_bootstrap: false,
            disable_bootstrap_listener: false,
            disable_bootstrap_bulk_pull_server: false,
            disable_bootstrap_bulk_push_client: false,
            disable_ongoing_bootstrap: false,
            disable_ascending_bootstrap: false,
            disable_rep_crawler: false,
            disable_request_loop: false,
            disable_tcp_realtime: false,
            disable_providing_telemetry_metrics: false,
            disable_ongoing_telemetry_requests: false,
            disable_block_processor_unchecked_deletion: false,
            disable_block_processor_republishing: false,
            allow_bootstrap_peers_duplicates: false,
            disable_max_peers_per_ip: false,
            disable_max_peers_per_subnetwork: false,
            force_use_write_queue: false,
            disable_search_pending: false,
            enable_pruning: false,
            fast_bootstrap: false,
            read_only: false,
            disable_connection_cleanup: false,
            generate_cache: GenerateCacheFlags::new(),
            inactive_node: false,
            block_processor_batch_size: BlockProcessorConfig::default().batch_size,
            block_processor_full_size: BlockProcessorConfig::default().full_size,
            block_processor_verification_size: 0,
            vote_processor_capacity: 144 * 1024,
            bootstrap_interval: 0,
        }
    }

    pub fn set_config_overrides(&mut self, config_overrides: Vec<String>) {
        self.config_overrides = config_overrides;
    }

    pub fn set_rpc_config_overrides(&mut self, rpc_config_overrides: Vec<String>) {
        self.rpc_config_overrides = rpc_config_overrides;
    }

    pub fn set_disable_activate_successors(&mut self, value: bool) {
        self.disable_activate_successors = value;
    }

    pub fn set_disable_backup(&mut self, value: bool) {
        self.disable_backup = value;
    }

    pub fn set_disable_lazy_bootstrap(&mut self, value: bool) {
        self.disable_lazy_bootstrap = value;
    }

    pub fn set_disable_legacy_bootstrap(&mut self, value: bool) {
        self.disable_legacy_bootstrap = value;
    }

    pub fn set_disable_wallet_bootstrap(&mut self, value: bool) {
        self.disable_wallet_bootstrap = value;
    }

    pub fn set_disable_bootstrap_listener(&mut self, value: bool) {
        self.disable_bootstrap_listener = value;
    }

    pub fn set_disable_bootstrap_bulk_pull_server(&mut self, value: bool) {
        self.disable_bootstrap_bulk_pull_server = value;
    }

    pub fn set_disable_bootstrap_bulk_push_client(&mut self, value: bool) {
        self.disable_bootstrap_bulk_push_client = value;
    }

    pub fn set_disable_ongoing_bootstrap(&mut self, value: bool) {
        self.disable_ongoing_bootstrap = value;
    }

    pub fn set_disable_ascending_bootstrap(&mut self, value: bool) {
        self.disable_ascending_bootstrap = value;
    }

    pub fn set_disable_rep_crawler(&mut self, value: bool) {
        self.disable_rep_crawler = value;
    }

    pub fn set_disable_request_loop(&mut self, value: bool) {
        self.disable_request_loop = value;
    }

    pub fn set_disable_tcp_realtime(&mut self, value: bool) {
        self.disable_tcp_realtime = value;
    }

    pub fn set_disable_providing_telemetry_metrics(&mut self, value: bool) {
        self.disable_providing_telemetry_metrics = value;
    }

    pub fn set_disable_ongoing_telemetry_requests(&mut self, value: bool) {
        self.disable_ongoing_telemetry_requests = value;
    }

    pub fn set_disable_block_processor_unchecked_deletion(&mut self, value: bool) {
        self.disable_block_processor_unchecked_deletion = value;
    }

    pub fn set_disable_block_processor_republishing(&mut self, value: bool) {
        self.disable_block_processor_republishing = value;
    }

    pub fn set_allow_bootstrap_peers_duplicates(&mut self, value: bool) {
        self.allow_bootstrap_peers_duplicates = value;
    }

    pub fn set_enable_pruning(&mut self, value: bool) {
        self.enable_pruning = value;
    }

    pub fn set_fast_bootstrap(&mut self, value: bool) {
        self.fast_bootstrap = value;
    }

    pub fn set_block_processor_batch_size(&mut self, value: usize) {
        self.block_processor_batch_size = value;
    }

    pub fn set_block_processor_full_size(&mut self, value: usize) {
        self.block_processor_full_size = value;
    }

    pub fn set_block_processor_verification_size(&mut self, value: usize) {
        self.block_processor_verification_size = value;
    }

    pub fn set_vote_processor_capacity(&mut self, value: usize) {
        self.vote_processor_capacity = value;
    }

    pub fn set_inactive_node(&mut self, value: bool) {
        self.inactive_node = value;
    }

    pub fn set_read_only(&mut self, value: bool) {
        self.read_only = value;
    }
}

impl Default for NodeFlags {
    fn default() -> Self {
        Self::new()
    }
}
