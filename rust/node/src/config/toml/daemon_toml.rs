use super::{NodeRpcToml, NodeToml, OpenclToml};
use crate::config::DaemonConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct DaemonToml {
    pub node: Option<NodeToml>,
    pub opencl: Option<OpenclToml>,
    pub rpc: Option<NodeRpcToml>,
}

impl DaemonConfig {
    pub fn merge_toml(&mut self, toml: &DaemonToml) {
        if let Some(node_toml) = &toml.node {
            self.node.merge_toml(node_toml);
        }
        if let Some(opencl) = &toml.opencl {
            if let Some(enable) = opencl.enable {
                self.opencl_enable = enable;
            }
            self.opencl.merge_toml(opencl);
        }
        if let Some(rpc) = &toml.rpc {
            if let Some(enable) = rpc.enable {
                self.rpc_enable = enable;
            }
            self.rpc.merge_toml(rpc);
        }
    }
}

impl From<&DaemonConfig> for DaemonToml {
    fn from(config: &DaemonConfig) -> Self {
        Self {
            node: Some((&config.node).into()),
            rpc: Some(config.into()),
            opencl: Some(config.into()),
        }
    }
}

impl From<&DaemonConfig> for NodeRpcToml {
    fn from(config: &DaemonConfig) -> Self {
        Self {
            enable: Some(config.rpc_enable),
            enable_sign_hash: Some(config.rpc.enable_sign_hash),
            child_process: Some((&config.rpc.child_process).into()),
        }
    }
}

impl From<&DaemonConfig> for OpenclToml {
    fn from(config: &DaemonConfig) -> Self {
        Self {
            enable: Some(config.opencl_enable),
            platform: Some(config.opencl.platform),
            device: Some(config.opencl.device),
            threads: Some(config.opencl.threads),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{DaemonConfig, DaemonToml},
        NetworkParams,
    };
    use rsnano_core::Networks;
    use std::path::PathBuf;

    static DEFAULT_TOML_STR: &str = r#"[node]
        allow_local_peers = true
        background_threads = 8
        backlog_scan_batch_size = 10000
        backlog_scan_frequency = 10
        backup_before_upgrade = false
        bandwidth_limit = 10485760
        bandwidth_limit_burst_ratio = 3.0
        block_processor_batch_max_time = 500
        bootstrap_bandwidth_burst_ratio = 1.0
        bootstrap_bandwidth_limit = 5242880
        bootstrap_connections = 4
        bootstrap_connections_max = 64
        bootstrap_fraction_numerator = 1
        bootstrap_frontier_request_count = 1048576
        bootstrap_initiator_threads = 1
        bootstrap_serving_threads = 1
        confirming_set_batch_time = 250
        enable_voting = false
        external_address = "::"
        external_port = 0
        frontiers_confirmation = "auto"
        io_threads = 8
        max_queued_requests = 512
        max_unchecked_blocks = 65536
        max_work_generate_multiplier = 64.0
        network_threads = 8
        online_weight_minimum = "60000000000000000000000000000000000000"
        password_fanout = 1024
        peering_port = 54000
        pow_sleep_interval = 0
        preconfigured_peers = ["peering-beta.nano.org"]
        preconfigured_representatives = ["nano_1defau1t9off1ine9rep99999999999999999999999999999999wgmuzxxy"]
        receive_minimum = "1000000000000000000000000"
        rep_crawler_weight_minimum = "340282366920938463463374607431768211455"
        representative_vote_weight_minimum = "10000000000000000000000000000000"
        request_aggregator_threads = 8
        signature_checker_threads = 4
        tcp_incoming_connections_max = 2048
        tcp_io_timeout = 15
        unchecked_cutoff_time = 14400
        use_memory_pools = true
        vote_generator_delay = 100
        vote_generator_threshold = 3
        vote_minimum = "1000000000000000000000000000000000"
        work_peers = []
        work_threads = 8

        [node.active_elections]
        confirmation_cache = 65536
        confirmation_history_size = 2048
        hinted_limit_percentage = 20
        optimistic_limit_percentage = 10
        size = 5000

        [node.block_processor]
        max_peer_queue = 128
        max_system_queue = 16384
        priority_bootstrap = 8
        priority_live = 1
        priority_local = 16

        [node.bootstrap_ascending]
        block_wait_count = 1000
        database_requests_limit = 1024
        pull_count = 128
        requests_limit = 64
        throttle_coefficient = 16
        throttle_wait = 100
        timeout = 3000

        [node.bootstrap_ascending.account_sets]
        blocking_max = 262144
        consideration_count = 4
        cooldown = 3000
        priorities_max = 262144

        [node.bootstrap_server]
        batch_size = 64
        max_queue = 16
        threads = 1

        [node.diagnostics.txn_tracking]
        enable = false
        ignore_writes_below_block_processor_max_time = true
        min_read_txn_time = 5000
        min_write_txn_time = 500

        [node.experimental]
        max_pruning_age = 300
        max_pruning_depth = 0
        secondary_work_peers = ["127.0.0.1:8076"]

        [node.httpcallback]
        address = ""
        port = 0
        target = ""

        [node.ipc.flatbuffers]
        skip_unexpected_fields_in_json = true
        verify_buffers = true

        [node.ipc.local]
        allow_unsafe = false
        enable = false
        io_timeout = 15
        io_threads = -1
        path = "/tmp/nano"

        [node.ipc.tcp]
        enable = false
        io_timeout = 15
        io_threads = -1
        port = 56000

        [node.lmdb]
        map_size = 274877906944
        max_databases = 128
        sync = "always"

        [node.message_processor]
        max_queue = 64
        threads = 2

        [node.monitor]
        enable = true
        interval = 60

        [node.optimistic_scheduler]
        enable = true
        gap_threshold = 32
        max_size = 65536

        [node.hinted_scheduler]
       	enable = true
       	hinting_threshold = 10
       	check_interval = 1000
       	block_cooldown = 5000
       	vacancy_threshold = 20

        [node.priority_bucket]
        max_blocks = 8192
        max_elections = 150
        reserved_elections = 100

        [node.rep_crawler]
        query_timeout = 60000

        [node.request_aggregator]
        batch_size = 16
        max_queue = 128
        threads = 4

        [node.statistics]
        max_samples = 16384

        [node.statistics.log]
        filename_counters = "counters.stat"
        filename_samples = "samples.stat"
        headers = true
        interval_counters = 0
        interval_samples = 0
        rotation_count = 100

        [node.vote_cache]
        age_cutoff = 900
        max_size = 65536
        max_voters = 64

        [node.vote_processor]
        batch_size = 1024
        max_non_pr_queue = 32
        max_pr_queue = 256
        pr_priority = 3
        threads = 4

        [node.websocket]
        address = "::1"
        enable = false
        port = 57000

        [opencl]
        device = 0
        enable = false
        platform = 0
        threads = 1048576

        [rpc]
        enable = false
        enable_sign_hash = false

        [rpc.child_process]
        enable = false
        rpc_path = "/home/foo/nano_rpc""#;

    static MODIFIED_TOML_STR: &str = r#"[node]
        allow_local_peers = false
        backup_before_upgrade = true
        bandwidth_limit = 999
        bandwidth_limit_burst_ratio = 999.9
        bootstrap_bandwidth_limit = 999
        bootstrap_bandwidth_burst_ratio = 999.9
        block_processor_batch_max_time = 999
        bootstrap_connections = 999
        bootstrap_connections_max = 999
        bootstrap_initiator_threads = 999
        bootstrap_serving_threads = 999
        bootstrap_frontier_request_count = 9999
        bootstrap_fraction_numerator = 999
        confirming_set_batch_time = 999
        enable_voting = true
        external_address = "0:0:0:0:0:ffff:7f01:101"
        external_port = 999
        io_threads = 999
        max_queued_requests = 999
        network_threads = 999
        background_threads = 999
        online_weight_minimum = "999"
        representative_vote_weight_minimum = "999"
        rep_crawler_weight_minimum = "999"
        password_fanout = 999
        peering_port = 999
        pow_sleep_interval = 999
        preconfigured_peers = ["dev.org"]
        preconfigured_representatives = ["nano_3arg3asgtigae3xckabaaewkx3bzsh7nwz7jkmjos79ihyaxwphhm6qgjps4"]
        receive_minimum = "999"
        signature_checker_threads = 999
        tcp_incoming_connections_max = 999
        tcp_io_timeout = 999
        unchecked_cutoff_time = 999
        use_memory_pools = false
        vote_generator_delay = 999
        vote_generator_threshold = 9
        vote_minimum = "999"
        work_peers = ["dev.org:999"]
        work_threads = 999
        max_work_generate_multiplier = 999
        request_aggregator_threads = 999
        max_unchecked_blocks = 999
        frontiers_confirmation = "always"
        backlog_scan_batch_size = 999
        backlog_scan_frequency = 999

        [node.block_processor]
        max_peer_queue = 999
        max_system_queue = 999
        priority_live = 999
        priority_bootstrap = 999
        priority_local = 999

        [node.active_elections]
        size = 999
        hinted_limit_percentage = 90
        optimistic_limit_percentage = 90
        confirmation_history_size = 999
        confirmation_cache = 999

        [node.diagnostics.txn_tracking]
        enable = true
        ignore_writes_below_block_processor_max_time = false
        min_read_txn_time = 999
        min_write_txn_time = 999

        [node.httpcallback]
        address = "dev.org"
        port = 999
        target = "/dev"

        [node.priority_bucket]
        max_blocks = 999
        max_elections = 999
        reserved_elections = 999

        [node.rep_crawler]
        query_timeout = 999

        [node.monitor]
        enable = false
        interval = 999

        [node.ipc.local]
        allow_unsafe = true
        enable = true
        io_timeout = 999
        io_threads = 999
        path = "/tmp/dev"

        [node.ipc.tcp]
        enable = true
        io_timeout = 999
        io_threads = 999
        port = 999

        [node.ipc.flatbuffers]
        skip_unexpected_fields_in_json = false
        verify_buffers = false

        [node.statistics]
        max_samples = 999

        [node.statistics.log]
        filename_counters = "devcounters.stat"
        filename_samples = "devsamples.stat"
        headers = false
        interval_counters = 999
        interval_samples = 999
        rotation_count = 999

        [node.websocket]
        address = "0:0:0:0:0:ffff:7f01:101"
        enable = true
        port = 999

        [node.lmdb]
        sync = "nosync_safe"
        max_databases = 999
        map_size = 999

        [node.optimistic_scheduler]
        enable = false
        gap_threshold = 999
        max_size = 999

        [node.hinted_scheduler]
        enable = false
        hinting_threshold = 99
        check_interval = 999
        block_cooldown = 999
        vacancy_threshold = 99

        [node.experimental]
        secondary_work_peers = ["dev.org:998"]
        max_pruning_age = 999
        max_pruning_depth = 999

        [node.vote_cache]
        age_cutoff = 999
        max_size = 999
        max_voters = 999

        [node.vote_processor]
        max_pr_queue = 999
        max_non_pr_queue = 999
        pr_priority = 999
        threads = 999
        batch_size = 999

        [node.bootstrap_ascending]
        block_wait_count = 999
        database_requests_limit = 999
        pull_count = 999
        requests_limit = 999
        throttle_coefficient = 999
        throttle_wait = 999
        timeout = 999

        [node.bootstrap_ascending.account_sets]
        blocking_max = 999
        consideration_count = 999
        cooldown = 999
        priorities_max = 999

        [node.bootstrap_server]
        max_queue = 999
        threads = 999
        batch_size = 999

        [node.request_aggregator]
        max_queue = 999
        threads = 999
        batch_size = 999

        [node.message_processor]
        threads = 999
        max_queue = 999

        [opencl]
        device = 999
        enable = true
        platform = 999
        threads = 999

        [rpc]
        enable = true
        enable_sign_hash = true

        [rpc.child_process]
        enable = true
        rpc_path = "/dev/nano_rpc""#;

    #[test]
    fn deserialize_defaults() {
        let deserialized_toml: DaemonToml = toml::from_str(&DEFAULT_TOML_STR).unwrap();
        let default_daemon_config = create_default_daemon_config();

        let mut deserialized_daemon_config = create_default_daemon_config();
        deserialized_daemon_config.merge_toml(&deserialized_toml);

        assert_eq!(&deserialized_daemon_config, &default_daemon_config);
    }

    #[test]
    fn deserialize_no_defaults() {
        let daemon_toml: DaemonToml =
            toml::from_str(MODIFIED_TOML_STR).expect("Failed to deserialize TOML");

        let mut deserialized_daemon_config = create_default_daemon_config();
        deserialized_daemon_config.merge_toml(&daemon_toml);

        let default_daemon_config = create_default_daemon_config();

        // Node section
        assert_ne!(
            deserialized_daemon_config.node.allow_local_peers,
            default_daemon_config.node.allow_local_peers
        );
        assert_ne!(
            deserialized_daemon_config.node.backup_before_upgrade,
            default_daemon_config.node.backup_before_upgrade
        );
        assert_ne!(
            deserialized_daemon_config.node.bandwidth_limit,
            default_daemon_config.node.bandwidth_limit
        );
        assert_ne!(
            deserialized_daemon_config.node.bandwidth_limit_burst_ratio,
            default_daemon_config.node.bandwidth_limit_burst_ratio
        );
        assert_ne!(
            deserialized_daemon_config.node.bootstrap_bandwidth_limit,
            default_daemon_config.node.bootstrap_bandwidth_limit
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_bandwidth_burst_ratio,
            default_daemon_config.node.bootstrap_bandwidth_burst_ratio
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .block_processor_batch_max_time_ms,
            default_daemon_config.node.block_processor_batch_max_time_ms
        );
        assert_ne!(
            deserialized_daemon_config.node.bootstrap_connections,
            default_daemon_config.node.bootstrap_connections
        );
        assert_ne!(
            deserialized_daemon_config.node.bootstrap_connections_max,
            default_daemon_config.node.bootstrap_connections_max
        );
        assert_ne!(
            deserialized_daemon_config.node.bootstrap_initiator_threads,
            default_daemon_config.node.bootstrap_initiator_threads
        );
        assert_ne!(
            deserialized_daemon_config.node.bootstrap_serving_threads,
            default_daemon_config.node.bootstrap_serving_threads
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_frontier_request_count,
            default_daemon_config.node.bootstrap_frontier_request_count
        );
        assert_ne!(
            deserialized_daemon_config.node.bootstrap_fraction_numerator,
            default_daemon_config.node.bootstrap_fraction_numerator
        );
        assert_ne!(
            deserialized_daemon_config.node.confirming_set_batch_time,
            default_daemon_config.node.confirming_set_batch_time
        );
        assert_ne!(
            deserialized_daemon_config.node.enable_voting,
            default_daemon_config.node.enable_voting
        );
        assert_ne!(
            deserialized_daemon_config.node.external_address,
            default_daemon_config.node.external_address
        );
        assert_ne!(
            deserialized_daemon_config.node.external_port,
            default_daemon_config.node.external_port
        );
        assert_ne!(
            deserialized_daemon_config.node.io_threads,
            default_daemon_config.node.io_threads
        );
        assert_ne!(
            deserialized_daemon_config.node.max_queued_requests,
            default_daemon_config.node.max_queued_requests
        );
        assert_ne!(
            deserialized_daemon_config.node.network_threads,
            default_daemon_config.node.network_threads
        );
        assert_ne!(
            deserialized_daemon_config.node.background_threads,
            default_daemon_config.node.background_threads
        );
        assert_ne!(
            deserialized_daemon_config.node.online_weight_minimum,
            default_daemon_config.node.online_weight_minimum
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .representative_vote_weight_minimum,
            default_daemon_config
                .node
                .representative_vote_weight_minimum
        );
        assert_ne!(
            deserialized_daemon_config.node.rep_crawler_weight_minimum,
            default_daemon_config.node.rep_crawler_weight_minimum
        );
        assert_ne!(
            deserialized_daemon_config.node.password_fanout,
            default_daemon_config.node.password_fanout
        );
        assert_ne!(
            deserialized_daemon_config.node.peering_port,
            default_daemon_config.node.peering_port
        );
        assert_ne!(
            deserialized_daemon_config.node.pow_sleep_interval_ns,
            default_daemon_config.node.pow_sleep_interval_ns
        );
        assert_ne!(
            deserialized_daemon_config.node.preconfigured_peers,
            default_daemon_config.node.preconfigured_peers
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .preconfigured_representatives,
            default_daemon_config.node.preconfigured_representatives
        );
        assert_ne!(
            deserialized_daemon_config.node.receive_minimum,
            default_daemon_config.node.receive_minimum
        );
        assert_ne!(
            deserialized_daemon_config.node.signature_checker_threads,
            default_daemon_config.node.signature_checker_threads
        );
        assert_ne!(
            deserialized_daemon_config.node.tcp_incoming_connections_max,
            default_daemon_config.node.tcp_incoming_connections_max
        );
        assert_ne!(
            deserialized_daemon_config.node.tcp_io_timeout_s,
            default_daemon_config.node.tcp_io_timeout_s
        );
        assert_ne!(
            deserialized_daemon_config.node.unchecked_cutoff_time_s,
            default_daemon_config.node.unchecked_cutoff_time_s
        );
        assert_ne!(
            deserialized_daemon_config.node.use_memory_pools,
            default_daemon_config.node.use_memory_pools
        );
        assert_ne!(
            deserialized_daemon_config.node.vote_generator_delay_ms,
            default_daemon_config.node.vote_generator_delay_ms
        );
        assert_ne!(
            deserialized_daemon_config.node.vote_generator_threshold,
            default_daemon_config.node.vote_generator_threshold
        );
        assert_ne!(
            deserialized_daemon_config.node.vote_minimum,
            default_daemon_config.node.vote_minimum
        );
        assert_ne!(
            deserialized_daemon_config.node.work_peers,
            default_daemon_config.node.work_peers
        );
        assert_ne!(
            deserialized_daemon_config.node.work_threads,
            default_daemon_config.node.work_threads
        );
        assert_ne!(
            deserialized_daemon_config.node.max_work_generate_multiplier,
            default_daemon_config.node.max_work_generate_multiplier
        );
        assert_ne!(
            deserialized_daemon_config.node.request_aggregator_threads,
            default_daemon_config.node.request_aggregator_threads
        );
        assert_ne!(
            deserialized_daemon_config.node.max_unchecked_blocks,
            default_daemon_config.node.max_unchecked_blocks
        );
        assert_ne!(
            deserialized_daemon_config.node.frontiers_confirmation,
            default_daemon_config.node.frontiers_confirmation
        );
        assert_ne!(
            deserialized_daemon_config.node.backlog_scan_batch_size,
            default_daemon_config.node.backlog_scan_batch_size
        );
        assert_ne!(
            deserialized_daemon_config.node.backlog_scan_frequency,
            default_daemon_config.node.backlog_scan_frequency
        );

        // Block Processor section
        assert_ne!(
            deserialized_daemon_config
                .node
                .block_processor
                .max_peer_queue,
            default_daemon_config.node.block_processor.max_peer_queue
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .block_processor
                .max_system_queue,
            default_daemon_config.node.block_processor.max_system_queue
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .block_processor
                .priority_live,
            default_daemon_config.node.block_processor.priority_live
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .block_processor
                .priority_bootstrap,
            default_daemon_config
                .node
                .block_processor
                .priority_bootstrap
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .block_processor
                .priority_local,
            default_daemon_config.node.block_processor.priority_local
        );

        // Active Elections section
        assert_ne!(
            deserialized_daemon_config.node.active_elections.size,
            default_daemon_config.node.active_elections.size
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .active_elections
                .hinted_limit_percentage,
            default_daemon_config
                .node
                .active_elections
                .hinted_limit_percentage
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .active_elections
                .optimistic_limit_percentage,
            default_daemon_config
                .node
                .active_elections
                .optimistic_limit_percentage
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .active_elections
                .confirmation_history_size,
            default_daemon_config
                .node
                .active_elections
                .confirmation_history_size
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .active_elections
                .confirmation_cache,
            default_daemon_config
                .node
                .active_elections
                .confirmation_cache
        );

        // Diagnostics section
        assert_ne!(
            deserialized_daemon_config
                .node
                .diagnostics_config
                .txn_tracking
                .enable,
            default_daemon_config
                .node
                .diagnostics_config
                .txn_tracking
                .enable
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .diagnostics_config
                .txn_tracking
                .ignore_writes_below_block_processor_max_time,
            default_daemon_config
                .node
                .diagnostics_config
                .txn_tracking
                .ignore_writes_below_block_processor_max_time
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .diagnostics_config
                .txn_tracking
                .min_read_txn_time_ms,
            default_daemon_config
                .node
                .diagnostics_config
                .txn_tracking
                .min_read_txn_time_ms
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .diagnostics_config
                .txn_tracking
                .min_write_txn_time_ms,
            default_daemon_config
                .node
                .diagnostics_config
                .txn_tracking
                .min_write_txn_time_ms
        );

        // HTTP Callback section
        assert_ne!(
            deserialized_daemon_config.node.callback_address,
            default_daemon_config.node.callback_address
        );
        assert_ne!(
            deserialized_daemon_config.node.callback_port,
            default_daemon_config.node.callback_port
        );
        assert_ne!(
            deserialized_daemon_config.node.callback_target,
            default_daemon_config.node.callback_target
        );

        // Priority Bucket section
        assert_ne!(
            deserialized_daemon_config.node.priority_bucket.max_blocks,
            default_daemon_config.node.priority_bucket.max_blocks
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .priority_bucket
                .max_elections,
            default_daemon_config.node.priority_bucket.max_elections
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .priority_bucket
                .reserved_elections,
            default_daemon_config
                .node
                .priority_bucket
                .reserved_elections
        );

        // Rep Crawler section
        assert_ne!(
            deserialized_daemon_config.node.rep_crawler_query_timeout,
            default_daemon_config.node.rep_crawler_query_timeout
        );

        // Monitor section
        assert_ne!(
            deserialized_daemon_config.node.monitor.enabled,
            default_daemon_config.node.monitor.enabled
        );
        assert_ne!(
            deserialized_daemon_config.node.monitor.interval,
            default_daemon_config.node.monitor.interval
        );

        // IPC Local section
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .transport_domain
                .transport
                .allow_unsafe,
            default_daemon_config
                .node
                .ipc_config
                .transport_domain
                .transport
                .allow_unsafe
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .transport_domain
                .transport
                .enabled,
            default_daemon_config
                .node
                .ipc_config
                .transport_domain
                .transport
                .enabled
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .transport_domain
                .transport
                .io_timeout,
            default_daemon_config
                .node
                .ipc_config
                .transport_domain
                .transport
                .io_timeout
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .transport_domain
                .transport
                .io_threads,
            default_daemon_config
                .node
                .ipc_config
                .transport_domain
                .transport
                .io_threads
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .transport_domain
                .path,
            default_daemon_config.node.ipc_config.transport_domain.path
        );

        // IPC TCP section
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .transport_tcp
                .transport
                .enabled,
            default_daemon_config
                .node
                .ipc_config
                .transport_tcp
                .transport
                .enabled,
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .transport_tcp
                .transport
                .io_timeout,
            default_daemon_config
                .node
                .ipc_config
                .transport_tcp
                .transport
                .io_timeout
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .transport_tcp
                .transport
                .io_threads,
            default_daemon_config
                .node
                .ipc_config
                .transport_tcp
                .transport
                .io_threads
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .transport_tcp
                .port,
            default_daemon_config.node.ipc_config.transport_tcp.port
        );

        // IPC Flatbuffers section
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .flatbuffers
                .skip_unexpected_fields_in_json,
            default_daemon_config
                .node
                .ipc_config
                .flatbuffers
                .skip_unexpected_fields_in_json
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .ipc_config
                .flatbuffers
                .verify_buffers,
            default_daemon_config
                .node
                .ipc_config
                .flatbuffers
                .verify_buffers
        );

        // Statistics section
        assert_ne!(
            deserialized_daemon_config.node.stat_config.max_samples,
            default_daemon_config.node.stat_config.max_samples
        );

        // Statistics Log section
        assert_ne!(
            deserialized_daemon_config
                .node
                .stat_config
                .log_counters_filename,
            default_daemon_config.node.stat_config.log_counters_filename
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .stat_config
                .log_samples_filename,
            default_daemon_config.node.stat_config.log_samples_filename
        );
        assert_ne!(
            deserialized_daemon_config.node.stat_config.log_headers,
            default_daemon_config.node.stat_config.log_headers
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .stat_config
                .log_counters_interval,
            default_daemon_config.node.stat_config.log_counters_interval
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .stat_config
                .log_samples_interval,
            default_daemon_config.node.stat_config.log_samples_interval
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .stat_config
                .log_rotation_count,
            default_daemon_config.node.stat_config.log_rotation_count
        );

        // WebSocket section
        assert_ne!(
            deserialized_daemon_config.node.websocket_config.address,
            default_daemon_config.node.websocket_config.address
        );
        assert_ne!(
            deserialized_daemon_config.node.websocket_config.enabled,
            default_daemon_config.node.websocket_config.enabled
        );
        assert_ne!(
            deserialized_daemon_config.node.websocket_config.port,
            default_daemon_config.node.websocket_config.port
        );

        // LMDB section
        assert_ne!(
            deserialized_daemon_config.node.lmdb_config.sync,
            default_daemon_config.node.lmdb_config.sync
        );
        assert_ne!(
            deserialized_daemon_config.node.lmdb_config.max_databases,
            default_daemon_config.node.lmdb_config.max_databases
        );
        assert_ne!(
            deserialized_daemon_config.node.lmdb_config.map_size,
            default_daemon_config.node.lmdb_config.map_size
        );

        // Optimistic Scheduler section
        assert_ne!(
            deserialized_daemon_config.node.optimistic_scheduler.enabled,
            default_daemon_config.node.optimistic_scheduler.enabled
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .optimistic_scheduler
                .gap_threshold,
            default_daemon_config
                .node
                .optimistic_scheduler
                .gap_threshold
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .optimistic_scheduler
                .max_size,
            default_daemon_config.node.optimistic_scheduler.max_size
        );

        // Hinted Scheduler section
        assert_ne!(
            deserialized_daemon_config.node.hinted_scheduler.enabled,
            default_daemon_config.node.hinted_scheduler.enabled
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .hinted_scheduler
                .hinting_threshold_percent,
            default_daemon_config
                .node
                .hinted_scheduler
                .hinting_threshold_percent
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .hinted_scheduler
                .check_interval,
            default_daemon_config.node.hinted_scheduler.check_interval
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .hinted_scheduler
                .block_cooldown,
            default_daemon_config.node.hinted_scheduler.block_cooldown
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .hinted_scheduler
                .vacancy_threshold_percent,
            default_daemon_config
                .node
                .hinted_scheduler
                .vacancy_threshold_percent
        );

        // Vote Cache section
        assert_ne!(
            deserialized_daemon_config.node.vote_cache.age_cutoff,
            default_daemon_config.node.vote_cache.age_cutoff
        );
        assert_ne!(
            deserialized_daemon_config.node.vote_cache.max_size,
            default_daemon_config.node.vote_cache.max_size
        );
        assert_ne!(
            deserialized_daemon_config.node.vote_cache.max_voters,
            default_daemon_config.node.vote_cache.max_voters
        );

        // Vote Processor section
        assert_ne!(
            deserialized_daemon_config.node.vote_processor.max_pr_queue,
            default_daemon_config.node.vote_processor.max_pr_queue
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .vote_processor
                .max_non_pr_queue,
            default_daemon_config.node.vote_processor.max_non_pr_queue
        );
        assert_ne!(
            deserialized_daemon_config.node.vote_processor.pr_priority,
            default_daemon_config.node.vote_processor.pr_priority
        );
        assert_ne!(
            deserialized_daemon_config.node.vote_processor.threads,
            default_daemon_config.node.vote_processor.threads
        );
        assert_ne!(
            deserialized_daemon_config.node.vote_processor.batch_size,
            default_daemon_config.node.vote_processor.batch_size
        );

        // Bootstrap Ascending section
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .block_wait_count,
            default_daemon_config
                .node
                .bootstrap_ascending
                .block_wait_count
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .database_requests_limit,
            default_daemon_config
                .node
                .bootstrap_ascending
                .database_requests_limit
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .pull_count,
            default_daemon_config.node.bootstrap_ascending.pull_count
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .requests_limit,
            default_daemon_config
                .node
                .bootstrap_ascending
                .requests_limit
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .throttle_coefficient,
            default_daemon_config
                .node
                .bootstrap_ascending
                .throttle_coefficient
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .throttle_wait,
            default_daemon_config.node.bootstrap_ascending.throttle_wait
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .request_timeout,
            default_daemon_config
                .node
                .bootstrap_ascending
                .request_timeout
        );

        // Bootstrap Ascending Account Sets section
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .account_sets
                .blocking_max,
            default_daemon_config
                .node
                .bootstrap_ascending
                .account_sets
                .blocking_max
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .account_sets
                .consideration_count,
            default_daemon_config
                .node
                .bootstrap_ascending
                .account_sets
                .consideration_count
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .account_sets
                .cooldown,
            default_daemon_config
                .node
                .bootstrap_ascending
                .account_sets
                .cooldown
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .bootstrap_ascending
                .account_sets
                .priorities_max,
            default_daemon_config
                .node
                .bootstrap_ascending
                .account_sets
                .priorities_max
        );

        // Bootstrap Server section
        assert_ne!(
            deserialized_daemon_config.node.bootstrap_server.max_queue,
            default_daemon_config.node.bootstrap_server.max_queue
        );
        assert_ne!(
            deserialized_daemon_config.node.bootstrap_server.threads,
            default_daemon_config.node.bootstrap_server.threads
        );
        assert_ne!(
            deserialized_daemon_config.node.bootstrap_server.batch_size,
            default_daemon_config.node.bootstrap_server.batch_size
        );

        // Request Aggregator section
        assert_ne!(
            deserialized_daemon_config.node.request_aggregator.max_queue,
            default_daemon_config.node.request_aggregator.max_queue
        );
        assert_ne!(
            deserialized_daemon_config.node.request_aggregator.threads,
            default_daemon_config.node.request_aggregator.threads
        );
        assert_ne!(
            deserialized_daemon_config
                .node
                .request_aggregator
                .batch_size,
            default_daemon_config.node.request_aggregator.batch_size
        );

        // Message Processor section
        assert_ne!(
            deserialized_daemon_config.node.message_processor.threads,
            default_daemon_config.node.message_processor.threads
        );
        assert_ne!(
            deserialized_daemon_config.node.message_processor.max_queue,
            default_daemon_config.node.message_processor.max_queue
        );

        // OpenCL section
        assert_ne!(
            deserialized_daemon_config.opencl.device,
            default_daemon_config.opencl.device
        );
        assert_ne!(
            deserialized_daemon_config.opencl_enable,
            default_daemon_config.opencl_enable
        );
        assert_ne!(
            deserialized_daemon_config.opencl.platform,
            default_daemon_config.opencl.platform
        );
        assert_ne!(
            deserialized_daemon_config.opencl.threads,
            default_daemon_config.opencl.threads
        );

        // RPC section
        assert_ne!(
            deserialized_daemon_config.rpc_enable,
            default_daemon_config.rpc_enable
        );
        assert_ne!(
            deserialized_daemon_config.rpc.enable_sign_hash,
            default_daemon_config.rpc.enable_sign_hash
        );

        // RPC Child Process section
        assert_ne!(
            deserialized_daemon_config.rpc.child_process.enable,
            default_daemon_config.rpc.child_process.enable
        );
        assert_ne!(
            deserialized_daemon_config.rpc.child_process.rpc_path,
            default_daemon_config.rpc.child_process.rpc_path
        );
    }

    #[test]
    fn deserialize_empty() {
        let toml_str = "";
        let daemon_toml: DaemonToml = toml::from_str(toml_str).expect("Failed to deserialize TOML");

        let mut deserialized_daemon_config = create_default_daemon_config();
        deserialized_daemon_config.merge_toml(&daemon_toml);
        let default_daemon_config = create_default_daemon_config();

        assert_eq!(&deserialized_daemon_config, &default_daemon_config);
    }

    fn create_default_daemon_config() -> DaemonConfig {
        let mut config = DaemonConfig::new(&NetworkParams::new(Networks::NanoBetaNetwork), 8);
        config.rpc.child_process.rpc_path = PathBuf::from("/home/foo/nano_rpc");
        config
    }

    #[test]
    fn serialize_defaults() {
        let default_daemon_config = create_default_daemon_config();
        let default_daemon_toml: DaemonToml = (&default_daemon_config).into();

        let serialized_toml = toml::to_string(&default_daemon_toml).unwrap();

        let default_toml_str_trimmed: String = DEFAULT_TOML_STR
            .lines()
            .map(str::trim)
            .collect::<Vec<&str>>()
            .join("\n");

        let serialized_toml_trimmed: String = serialized_toml
            .lines()
            .map(str::trim)
            .collect::<Vec<&str>>()
            .join("\n");

        assert_eq!(&serialized_toml_trimmed, &default_toml_str_trimmed);
    }
}
