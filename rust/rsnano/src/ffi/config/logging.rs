use crate::config::Logging;

#[repr(C)]
pub struct LoggingDto {
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

#[no_mangle]
pub unsafe extern "C" fn rsn_logging_create(dto: *mut LoggingDto) {
    let dto = &mut (*dto);
    let logging = Logging::new();
    fill_logging_dto(dto, &logging);
}

pub fn fill_logging_dto(dto: &mut LoggingDto, logging: &Logging) {
    dto.ledger_logging_value = logging.ledger_logging_value;
    dto.ledger_duplicate_logging_value = logging.ledger_duplicate_logging_value;
    dto.ledger_rollback_logging_value = logging.ledger_rollback_logging_value;
    dto.vote_logging_value = logging.vote_logging_value;
    dto.rep_crawler_logging_value = logging.rep_crawler_logging_value;
    dto.election_fork_tally_logging_value = logging.election_fork_tally_logging_value;
    dto.election_expiration_tally_logging_value = logging.election_expiration_tally_logging_value;
    dto.network_logging_value = logging.network_logging_value;
    dto.network_timeout_logging_value = logging.network_timeout_logging_value;
    dto.network_message_logging_value = logging.network_message_logging_value;
    dto.network_publish_logging_value = logging.network_publish_logging_value;
    dto.network_packet_logging_value = logging.network_packet_logging_value;
    dto.network_keepalive_logging_value = logging.network_keepalive_logging_value;
    dto.network_node_id_handshake_logging_value = logging.network_node_id_handshake_logging_value;
    dto.network_telemetry_logging_value = logging.network_telemetry_logging_value;
    dto.network_rejected_logging_value = logging.network_rejected_logging_value;
    dto.node_lifetime_tracing_value = logging.node_lifetime_tracing_value;
    dto.insufficient_work_logging_value = logging.insufficient_work_logging_value;
    dto.log_ipc_value = logging.log_ipc_value;
    dto.bulk_pull_logging_value = logging.bulk_pull_logging_value;
    dto.work_generation_time_value = logging.work_generation_time_value;
    dto.upnp_details_logging_value = logging.upnp_details_logging_value;
    dto.timing_logging_value = logging.timing_logging_value;
    dto.active_update_value = logging.active_update_value;
    dto.log_to_cerr_value = logging.log_to_cerr_value;
    dto.flush = logging.flush;
    dto.max_size = logging.max_size;
    dto.rotation_size = logging.rotation_size;
    dto.stable_log_filename = logging.stable_log_filename;
    dto.min_time_between_log_output_ms = logging.min_time_between_log_output_ms;
    dto.single_line_record_value = logging.single_line_record_value;
    dto.election_result_logging_value = logging.election_result_logging_value;
}

impl From<&LoggingDto> for Logging {
    fn from(dto: &LoggingDto) -> Self {
        Self {
            ledger_logging_value: dto.ledger_logging_value,
            ledger_duplicate_logging_value: dto.ledger_duplicate_logging_value,
            ledger_rollback_logging_value: dto.ledger_rollback_logging_value,
            vote_logging_value: dto.vote_logging_value,
            rep_crawler_logging_value: dto.rep_crawler_logging_value,
            election_fork_tally_logging_value: dto.election_fork_tally_logging_value,
            election_expiration_tally_logging_value: dto.election_expiration_tally_logging_value,
            network_logging_value: dto.network_logging_value,
            network_timeout_logging_value: dto.network_timeout_logging_value,
            network_message_logging_value: dto.network_message_logging_value,
            network_publish_logging_value: dto.network_publish_logging_value,
            network_packet_logging_value: dto.network_packet_logging_value,
            network_keepalive_logging_value: dto.network_keepalive_logging_value,
            network_node_id_handshake_logging_value: dto.network_node_id_handshake_logging_value,
            network_telemetry_logging_value: dto.network_telemetry_logging_value,
            network_rejected_logging_value: dto.network_rejected_logging_value,
            node_lifetime_tracing_value: dto.node_lifetime_tracing_value,
            insufficient_work_logging_value: dto.insufficient_work_logging_value,
            log_ipc_value: dto.log_ipc_value,
            bulk_pull_logging_value: dto.bulk_pull_logging_value,
            work_generation_time_value: dto.work_generation_time_value,
            upnp_details_logging_value: dto.upnp_details_logging_value,
            timing_logging_value: dto.timing_logging_value,
            active_update_value: dto.active_update_value,
            log_to_cerr_value: dto.log_to_cerr_value,
            flush: dto.flush,
            max_size: dto.max_size,
            rotation_size: dto.rotation_size,
            stable_log_filename: dto.stable_log_filename,
            min_time_between_log_output_ms: dto.min_time_between_log_output_ms,
            single_line_record_value: dto.single_line_record_value,
            election_result_logging_value: dto.election_result_logging_value,
        }
    }
}

impl From<LoggingDto> for Logging {
    fn from(dto: LoggingDto) -> Self {
        (&dto).into()
    }
}
