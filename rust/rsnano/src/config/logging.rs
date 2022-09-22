use anyhow::Result;

use crate::utils::TomlWriter;

pub struct Logging {
    pub ledger_logging_value: bool,
    pub ledger_duplicate_logging_value: bool,
    pub ledger_rollback_logging_value: bool,
    pub vote_logging_value: bool,
    pub rep_crawler_logging_value: bool,
    pub election_fork_tally_logging_value: bool,
    pub election_expiration_tally_logging_value: bool,
    pub network_logging_value: bool,
    pub network_timeout_logging_value: bool,
    pub network_message_logging_value: bool,
    pub network_publish_logging_value: bool,
    pub network_packet_logging_value: bool,
    pub network_keepalive_logging_value: bool,
    pub network_node_id_handshake_logging_value: bool,
    pub network_telemetry_logging_value: bool,
    pub network_rejected_logging_value: bool,
    pub node_lifetime_tracing_value: bool,
    pub insufficient_work_logging_value: bool,
    pub log_ipc_value: bool,
    pub bulk_pull_logging_value: bool,
    pub work_generation_time_value: bool,
    pub upnp_details_logging_value: bool,
    pub timing_logging_value: bool,
    pub active_update_value: bool,
    pub log_to_cerr_value: bool,
    pub flush: bool,
    pub max_size: usize,
    pub rotation_size: usize,
    pub stable_log_filename: bool,
    pub min_time_between_log_output_ms: i64,
    pub single_line_record_value: bool,
    pub election_result_logging_value: bool,
}

impl Logging {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_bool(
            "ledger",
            self.ledger_logging_value,
            "Log ledger related messages.\ntype:bool",
        )?;
        toml.put_bool(
            "ledger_duplicate",
            self.ledger_duplicate_logging_value,
            "Log when a duplicate block is attempted inserted into the ledger.\ntype:bool",
        )?;
        toml.put_bool(
            "ledger_rollback",
            self.election_fork_tally_logging_value,
            "Log when a block is replaced in the ledger.\ntype:bool",
        )?;
        toml.put_bool("vote", self.vote_logging_value, "Vote logging. Enabling this option leads to a high volume.\nof log messages which may affect node performance.\ntype:bool")?;
        toml.put_bool("rep_crawler", self.rep_crawler_logging_value, "Rep crawler logging. Enabling this option leads to a high volume.\nof log messages which may affect node performance.\ntype:bool")?;
        toml.put_bool(
            "election_expiration",
            self.election_expiration_tally_logging_value,
            "Log election tally on expiration.\ntype:bool",
        )?;
        toml.put_bool(
            "election_fork",
            self.election_fork_tally_logging_value,
            "Log election tally when more than one block is seen.\ntype:bool",
        )?;
        toml.put_bool(
            "network",
            self.network_logging_value,
            "Log network related messages.\ntype:bool",
        )?;
        toml.put_bool(
            "network_timeout",
            self.network_timeout_logging_value,
            "Log TCP timeouts.\ntype:bool",
        )?;
        toml.put_bool(
            "network_message",
            self.network_message_logging_value,
            "Log network errors and message details.\ntype:bool",
        )?;
        toml.put_bool(
            "network_publish",
            self.network_publish_logging_value,
            "Log publish related network messages.\ntype:bool",
        )?;
        toml.put_bool(
            "network_packet",
            self.network_packet_logging_value,
            "Log network packet activity.\ntype:bool",
        )?;
        toml.put_bool(
            "network_keepalive",
            self.network_keepalive_logging_value,
            "Log keepalive related messages.\ntype:bool",
        )?;
        toml.put_bool(
            "network_node_id_handshake",
            self.network_node_id_handshake_logging_value,
            "Log node-id handshake related messages.\ntype:bool",
        )?;
        toml.put_bool(
            "network_telemetry",
            self.network_telemetry_logging_value,
            "Log telemetry related messages.\ntype:bool",
        )?;
        toml.put_bool(
            "network_rejected",
            self.network_rejected_logging_value,
            "Log message when a connection is rejected.\ntype:bool",
        )?;
        toml.put_bool(
            "node_lifetime_tracing",
            self.node_lifetime_tracing_value,
            "Log node startup and shutdown messages.\ntype:bool",
        )?;
        toml.put_bool(
            "insufficient_work",
            self.insufficient_work_logging_value,
            "Log if insufficient work is detected.\ntype:bool",
        )?;
        toml.put_bool(
            "log_ipc",
            self.log_ipc_value,
            "Log IPC related activity.\ntype:bool",
        )?;
        toml.put_bool(
            "bulk_pull",
            self.bulk_pull_logging_value,
            "Log bulk pull errors and messages.\ntype:bool",
        )?;
        toml.put_bool(
            "work_generation_time",
            self.work_generation_time_value,
            "Log work generation timing information.\ntype:bool",
        )?;
        toml.put_bool("upnp_details", self.upnp_details_logging_value, "Log UPNP discovery details..\nWarning: this may include information.\nabout discovered devices, such as product identification. Please review before sharing logs.\ntype:bool")?;
        toml.put_bool(
            "timing",
            self.timing_logging_value,
            "Log detailed timing information for various node operations.\ntype:bool",
        )?;
        toml.put_bool(
            "active_update",
            self.active_update_value,
            "Log when a block is updated while in active transactions.\ntype:bool",
        )?;
        toml.put_bool("election_result", self.election_result_logging_value, "Log election result when cleaning up election from active election container.\ntype:bool")?;
        toml.put_bool("log_to_cerr", self.log_to_cerr_value, "Log to standard error in addition to the log file. Not recommended for production systems.\ntype:bool")?;
        toml.put_usize(
            "max_size",
            self.max_size,
            "Maximum log file size in bytes.\ntype:uint64",
        )?;
        toml.put_usize(
            "rotation_size",
            self.rotation_size,
            "Log file rotation size in character count.\ntype:uint64",
        )?;
        toml.put_bool("flush", self.flush, "If enabled, immediately flush new entries to log file.\nWarning: this may negatively affect logging performance.\ntype:bool")?;
        toml.put_i64("min_time_between_output", self.min_time_between_log_output_ms, "Minimum time that must pass for low priority entries to be logged.\nWarning: decreasing this value may result in a very large amount of logs.\ntype:milliseconds")?;
        toml.put_bool(
            "single_line_record",
            self.single_line_record_value,
            "Keep log entries on single lines.\ntype:bool",
        )?;
        toml.put_bool("stable_log_filename", self.stable_log_filename, "Append to log/node.log without a timestamp in the filename.\nThe file is not emptied on startup if it exists, but appended to.\ntype:bool")?;

        Ok(())
    }
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            ledger_logging_value: false,
            ledger_duplicate_logging_value: false,
            ledger_rollback_logging_value: false,
            vote_logging_value: false,
            rep_crawler_logging_value: false,
            election_fork_tally_logging_value: false,
            election_expiration_tally_logging_value: false,
            network_logging_value: true,
            network_timeout_logging_value: false,
            network_message_logging_value: false,
            network_publish_logging_value: false,
            network_packet_logging_value: false,
            network_keepalive_logging_value: false,
            network_node_id_handshake_logging_value: false,
            network_telemetry_logging_value: false,
            network_rejected_logging_value: false,
            node_lifetime_tracing_value: false,
            insufficient_work_logging_value: true,
            log_ipc_value: true,
            bulk_pull_logging_value: false,
            work_generation_time_value: true,
            upnp_details_logging_value: false,
            timing_logging_value: false,
            active_update_value: false,
            log_to_cerr_value: false,
            flush: true,
            max_size: 128 * 1024 * 1024,
            rotation_size: 4 * 1024 * 1024,
            stable_log_filename: false,
            min_time_between_log_output_ms: 5,
            single_line_record_value: false,
            election_result_logging_value: false,
        }
    }
}
