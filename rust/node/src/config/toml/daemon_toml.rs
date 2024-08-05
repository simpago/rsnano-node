use super::{NodeRpcToml, NodeToml, OpenclToml};
use crate::config::DaemonConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct DaemonToml {
    pub node: Option<NodeToml>,
    pub opencl: Option<OpenclToml>,
    pub rpc: Option<NodeRpcToml>,
}

impl From<&DaemonToml> for DaemonConfig {
    fn from(toml: &DaemonToml) -> Self {
        let mut config = DaemonConfig::default();
        if let Some(node_toml) = &toml.node {
            config.node = node_toml.into();
        }
        if let Some(opencl) = &toml.opencl {
            if let Some(enable) = opencl.enable {
                config.opencl_enable = enable;
            }
            config.opencl = opencl.into();
        }
        if let Some(rpc) = &toml.rpc {
            if let Some(enable) = rpc.enable {
                config.rpc_enable = enable;
            }
            config.rpc = rpc.into();
        }
        config
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

impl Default for DaemonToml {
    fn default() -> Self {
        Self {
            node: Some(NodeToml::default()),
            opencl: Some(OpenclToml::default()),
            rpc: Some(NodeRpcToml::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{DaemonConfig, DaemonToml},
        nullable_fs::NullableFilesystem,
    };
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
        rpc_path = "/Users/ruimorais/rsnano/rust/../build/cargo/debug/deps/nano_rpc""#;

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

        let default_daemon_config = DaemonConfig::default();
        let deserialized_daemon_config: DaemonConfig = (&deserialized_toml).into();

        assert_eq!(&deserialized_daemon_config, &default_daemon_config);
    }

    #[test]
    fn deserialize_no_defaults() {
        let path: PathBuf = "node-config.toml".into();

        let fs = NullableFilesystem::null_builder()
            .read_to_string(&path, MODIFIED_TOML_STR.to_string())
            .finish();

        let toml_read = fs.read_to_string(&path).unwrap();

        let daemon_toml: DaemonToml =
            toml::from_str(&toml_read).expect("Failed to deserialize TOML");

        let deserialized_daemon_config: DaemonConfig = (&daemon_toml).into();

        let default_daemon_config = DaemonConfig::default();

        assert_ne!(&deserialized_daemon_config, &default_daemon_config);
    }

    #[test]
    fn deserialize_commented() {
        let path: PathBuf = "node-config.toml".into();

        let fs = NullableFilesystem::null_builder()
            .read_to_string(&path, comment_fields(MODIFIED_TOML_STR).to_string())
            .finish();

        let toml_read = fs.read_to_string(&path).unwrap();

        let daemon_toml: DaemonToml =
            toml::from_str(&toml_read).expect("Failed to deserialize TOML");

        let deserialized_daemon_config: DaemonConfig = (&daemon_toml).into();

        let default_daemon_config = DaemonConfig::default();

        assert_eq!(&deserialized_daemon_config, &default_daemon_config);
    }

    #[test]
    fn deserialize_empty() {
        let path: PathBuf = "node-config.toml".into();

        let toml_str = r#""#;

        let fs = NullableFilesystem::null_builder()
            .read_to_string(&path, toml_str.to_string())
            .finish();

        let toml_read = fs.read_to_string(&path).unwrap();

        let daemon_toml: DaemonToml =
            toml::from_str(&toml_read).expect("Failed to deserialize TOML");

        let deserialized_daemon_config: DaemonConfig = (&daemon_toml).into();

        let default_daemon_config = DaemonConfig::default();

        assert_eq!(&deserialized_daemon_config, &default_daemon_config);
    }

    #[test]
    fn serialize_defaults() {
        let default_daemon_config = DaemonConfig::default();

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

    fn comment_fields(toml_str: &str) -> String {
        let mut result = String::new();
        let mut in_header = false;

        for line in toml_str.lines() {
            if line.trim().is_empty() {
                result.push_str("\n");
                continue;
            }

            if line.trim().starts_with("[") && line.trim().ends_with("]") {
                if in_header {
                    result.push_str("\n");
                }
                result.push_str(line);
                result.push_str("\n");
                in_header = true;
            } else {
                if in_header {
                    result.push_str("# ");
                    result.push_str(line);
                    result.push_str("\n");
                }
            }
        }

        result
    }
}
