use anyhow::Result;
use bounded_vec_deque::BoundedVecDeque;
use num::FromPrimitive;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant, SystemTime},
};

use crate::messages::MessageType;

use super::histogram::StatHistogram;
use super::{FileWriter, StatConfig, StatLogSink};

/// Value and wall time of measurement
#[derive(Clone)]
pub struct StatDatapoint {
    /// Value of the sample interval
    value: u64,
    /// When the sample was added. This is wall time (system_clock), suitable for display purposes.
    timestamp: SystemTime, //todo convert back to Instant
}

impl Default for StatDatapoint {
    fn default() -> Self {
        Self {
            value: 0,
            timestamp: SystemTime::now(),
        }
    }
}

impl StatDatapoint {
    pub fn new() -> Self {
        Default::default()
    }

    pub(crate) fn get_value(&self) -> u64 {
        self.value
    }

    pub(crate) fn set_value(&mut self, value: u64) {
        self.value = value;
    }

    pub(crate) fn get_timestamp(&self) -> SystemTime {
        self.timestamp
    }

    pub(crate) fn set_timestamp(&mut self, timestamp: SystemTime) {
        self.timestamp = timestamp;
    }

    pub(crate) fn add(&mut self, addend: u64, update_timestamp: bool) {
        self.value += addend;
        if update_timestamp {
            self.timestamp = SystemTime::now();
        }
    }
}

pub struct StatEntry {
    /// Sample interval in milliseconds. If 0, sampling is disabled.
    pub sample_interval: usize,

    /// Value within the current sample interval
    pub sample_current: StatDatapoint,

    /// Optional samples. Note that this doesn't allocate any memory unless sampling is configured, which sets the capacity.
    pub samples: Option<BoundedVecDeque<StatDatapoint>>,

    /// Counting value for this entry, including the time of last update. This is never reset and only increases.
    pub counter: StatDatapoint,

    /// Start time of current sample interval. This is a steady clock for measuring interval; the datapoint contains the wall time.
    pub sample_start_time: Instant,

    /// Optional histogram for this entry
    pub histogram: Option<StatHistogram>,
}

impl StatEntry {
    pub fn new(capacity: usize, interval: usize) -> Self {
        Self {
            sample_interval: interval,
            sample_current: StatDatapoint::new(),
            samples: if capacity > 0 {
                Some(BoundedVecDeque::new(capacity))
            } else {
                None
            },
            counter: StatDatapoint::new(),
            sample_start_time: Instant::now(),
            histogram: None,
        }
    }
}

/// Primary statistics type
#[repr(u8)]
#[derive(FromPrimitive)]
pub enum StatType {
    TrafficUdp,
    TrafficTcp,
    Error,
    Message,
    Block,
    Ledger,
    Rollback,
    Bootstrap,
    TcpServer,
    Vote,
    Election,
    HttpCallback,
    Peering,
    Ipc,
    Tcp,
    Udp,
    ConfirmationHeight,
    ConfirmationObserver,
    Drop,
    Aggregator,
    Requests,
    Filter,
    Telemetry,
    VoteGenerator,
    VoteCache,
    Hinting,
    BlockProcessor,
    BootstrapServer,
    Active,
    Backlog,
}

impl StatType {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatType::Ipc => "ipc",
            StatType::Block => "block",
            StatType::Bootstrap => "bootstrap",
            StatType::TcpServer => "tcp_server",
            StatType::Error => "error",
            StatType::HttpCallback => "http_callback",
            StatType::Ledger => "ledger",
            StatType::Tcp => "tcp",
            StatType::Udp => "udp",
            StatType::Peering => "peering",
            StatType::Rollback => "rollback",
            StatType::TrafficUdp => "traffic_udp",
            StatType::TrafficTcp => "traffic_tcp",
            StatType::Vote => "vote",
            StatType::Election => "election",
            StatType::Message => "message",
            StatType::ConfirmationObserver => "observer",
            StatType::ConfirmationHeight => "confirmation_height",
            StatType::Drop => "drop",
            StatType::Aggregator => "aggregator",
            StatType::Requests => "requests",
            StatType::Filter => "filter",
            StatType::Telemetry => "telemetry",
            StatType::VoteGenerator => "vote_generator",
            StatType::VoteCache => "vote_cache",
            StatType::Hinting => "hinting",
            StatType::BlockProcessor => "blockprocessor",
            StatType::BootstrapServer => "bootstrap_server",
            StatType::Active => "active",
            StatType::Backlog => "backlog",
        }
    }
}

// Optional detail type
#[repr(u8)]
#[derive(FromPrimitive)]
pub enum DetailType {
    All = 0,

    // common
    Loop,
    Total,

    // processing queue
    Queue,
    Overfill,
    Batch,

    // error specific
    BadSender,
    InsufficientWork,
    HttpCallback,
    UnreachableHost,
    InvalidNetwork,

    // confirmation_observer specific
    ActiveQuorum,
    ActiveConfHeight,
    InactiveConfHeight,

    // ledger, block, bootstrap
    Send,
    Receive,
    Open,
    Change,
    StateBlock,
    EpochBlock,
    Fork,
    Old,
    GapPrevious,
    GapSource,
    RollbackFailed,
    Progress,
    BadSignature,
    NegativeSpend,
    Unreceivable,
    GapEpochOpenPending,
    OpenedBurnAccount,
    BalanceMismatch,
    RepresentativeMismatch,
    BlockPosition,

    // message specific
    NotAType,
    Invalid,
    Keepalive,
    Publish,
    RepublishVote,
    ConfirmReq,
    ConfirmAck,
    NodeIdHandshake,
    TelemetryReq,
    TelemetryAck,
    AscPullReq,
    AscPullAck,

    // bootstrap, callback
    Initiate,
    InitiateLegacyAge,
    InitiateLazy,
    InitiateWalletLazy,

    // bootstrap specific
    BulkPull,
    BulkPullAccount,
    BulkPullDeserializeReceiveBlock,
    BulkPullErrorStartingRequest,
    BulkPullFailedAccount,
    BulkPullReceiveBlockFailure,
    BulkPullRequestFailure,
    BulkPush,
    FrontierReq,
    FrontierConfirmationFailed,
    FrontierConfirmationSuccessful,
    ErrorSocketClose,
    RequestUnderflow,

    // vote specific
    VoteValid,
    VoteReplay,
    VoteIndeterminate,
    VoteInvalid,
    VoteOverflow,

    // election specific
    VoteNew,
    VoteProcessed,
    VoteCached,
    LateBlock,
    LateBlockSeconds,
    ElectionStart,
    ElectionConfirmedAll,
    ElectionBlockConflict,
    ElectionDifficultyUpdate,
    ElectionDropExpired,
    ElectionDropOverflow,
    ElectionDropAll,
    ElectionRestart,
    ElectionConfirmed,
    ElectionNotConfirmed,
    ElectionHintedOverflow,
    ElectionHintedStarted,
    ElectionHintedConfirmed,
    ElectionHintedDrop,
    GenerateVote,
    GenerateVoteNormal,
    GenerateVoteFinal,

    // udp
    Blocking,
    Overflow,
    InvalidHeader,
    InvalidMessageType,
    InvalidKeepaliveMessage,
    InvalidPublishMessage,
    InvalidConfirmReqMessage,
    InvalidConfirmAckMessage,
    InvalidNodeIdHandshakeMessage,
    InvalidTelemetryReqMessage,
    InvalidTelemetryAckMessage,
    InvalidBulkPullMessage,
    InvalidBulkPullAccountMessage,
    InvalidFrontierReqMessage,
    InvalidAscPullReqMessage,
    InvalidAscPullAckMessage,
    MessageTooBig,
    OutdatedVersion,
    UdpMaxPerIp,
    UdpMaxPerSubnetwork,

    // tcp
    TcpAcceptSuccess,
    TcpAcceptFailure,
    TcpWriteDrop,
    TcpWriteNoSocketDrop,
    TcpExcluded,
    TcpMaxPerIp,
    TcpMaxPerSubnetwork,
    TcpSilentConnectionDrop,
    TcpIoTimeoutDrop,
    TcpConnectError,
    TcpReadError,
    TcpWriteError,

    // ipc
    Invocations,

    // peering
    Handshake,

    // confirmation height
    BlocksConfirmed,
    BlocksConfirmedUnbounded,
    BlocksConfirmedBounded,

    // [request] aggregator
    AggregatorAccepted,
    AggregatorDropped,

    // requests
    RequestsCachedHashes,
    RequestsGeneratedHashes,
    RequestsCachedVotes,
    RequestsGeneratedVotes,
    RequestsCachedLateHashes,
    RequestsCachedLateVotes,
    RequestsCannotVote,
    RequestsUnknown,

    // duplicate
    DuplicatePublish,

    // telemetry
    InvalidSignature,
    DifferentGenesisHash,
    NodeIdMismatch,
    RequestWithinProtectionCacheZone,
    NoResponseReceived,
    UnsolicitedTelemetryAck,
    FailedSendTelemetryReq,

    // vote generator
    GeneratorBroadcasts,
    GeneratorReplies,
    GeneratorRepliesDiscarded,
    GeneratorSpacing,

    // hinting
    Hinted,
    InsertFailed,
    MissingBlock,

    // bootstrap server
    Response,
    WriteDrop,
    WriteError,
    Blocks,
    Drop,
    BadCount,
    ResponseBlocks,
    ResponseAccountInfo,
    ChannelFull,

    // backlog
    Activated,
}

impl DetailType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DetailType::All => "all",
            DetailType::Loop => "loop",
            DetailType::Total => "total",
            DetailType::Queue => "queue",
            DetailType::Overfill => "overfill",
            DetailType::Batch => "batch",
            DetailType::BadSender => "bad_sender",
            DetailType::BulkPull => "bulk_pull",
            DetailType::BulkPullAccount => "bulk_pull_account",
            DetailType::BulkPullDeserializeReceiveBlock => "bulk_pull_deserialize_receive_block",
            DetailType::BulkPullErrorStartingRequest => "bulk_pull_error_starting_request",
            DetailType::BulkPullFailedAccount => "bulk_pull_failed_account",
            DetailType::BulkPullReceiveBlockFailure => "bulk_pull_receive_block_failure",
            DetailType::BulkPullRequestFailure => "bulk_pull_request_failure",
            DetailType::BulkPush => "bulk_push",
            DetailType::ActiveQuorum => "observer_confirmation_active_quorum",
            DetailType::ActiveConfHeight => "observer_confirmation_active_conf_height",
            DetailType::InactiveConfHeight => "observer_confirmation_inactive",
            DetailType::ErrorSocketClose => "error_socket_close",
            DetailType::RequestUnderflow => "request_underflow",
            DetailType::Change => "change",
            DetailType::ConfirmAck => "confirm_ack",
            DetailType::NodeIdHandshake => "node_id_handshake",
            DetailType::ConfirmReq => "confirm_req",
            DetailType::Fork => "fork",
            DetailType::Old => "old",
            DetailType::GapPrevious => "gap_previous",
            DetailType::GapSource => "gap_source",
            DetailType::RollbackFailed => "rollback_failed",
            DetailType::Progress => "progress",
            DetailType::BadSignature => "bad_signature",
            DetailType::NegativeSpend => "negative_spend",
            DetailType::Unreceivable => "unreceivable",
            DetailType::GapEpochOpenPending => "gap_epoch_open_pending",
            DetailType::OpenedBurnAccount => "opened_burn_account",
            DetailType::BalanceMismatch => "balance_mismatch",
            DetailType::RepresentativeMismatch => "representative_mismatch",
            DetailType::BlockPosition => "block_position",
            DetailType::FrontierConfirmationFailed => "frontier_confirmation_failed",
            DetailType::FrontierConfirmationSuccessful => "frontier_confirmation_successful",
            DetailType::FrontierReq => "frontier_req",
            DetailType::Handshake => "handshake",
            DetailType::HttpCallback => "http_callback",
            DetailType::Initiate => "initiate",
            DetailType::InitiateLegacyAge => "initiate_legacy_age",
            DetailType::InitiateLazy => "initiate_lazy",
            DetailType::InitiateWalletLazy => "initiate_wallet_lazy",
            DetailType::InsufficientWork => "insufficient_work",
            DetailType::Invalid => "invalid",
            DetailType::Invocations => "invocations",
            DetailType::Keepalive => "keepalive",
            DetailType::NotAType => "not_a_type",
            DetailType::Open => "open",
            DetailType::Publish => "publish",
            DetailType::Receive => "receive",
            DetailType::RepublishVote => "republish_vote",
            DetailType::Send => "send",
            DetailType::TelemetryReq => "telemetry_req",
            DetailType::TelemetryAck => "telemetry_ack",
            DetailType::AscPullReq => "asc_pull_req",
            DetailType::AscPullAck => "asc_pull_ack",
            DetailType::StateBlock => "state_block",
            DetailType::EpochBlock => "epoch_block",
            DetailType::VoteValid => "vote_valid",
            DetailType::VoteReplay => "vote_replay",
            DetailType::VoteIndeterminate => "vote_indeterminate",
            DetailType::VoteInvalid => "vote_invalid",
            DetailType::VoteOverflow => "vote_overflow",
            DetailType::VoteNew => "vote_new",
            DetailType::VoteProcessed => "vote_processed",
            DetailType::VoteCached => "vote_cached",
            DetailType::LateBlock => "late_block",
            DetailType::LateBlockSeconds => "late_block_seconds",
            DetailType::ElectionStart => "election_start",
            DetailType::ElectionConfirmedAll => "election_confirmed_all",
            DetailType::ElectionBlockConflict => "election_block_conflict",
            DetailType::ElectionDifficultyUpdate => "election_difficulty_update",
            DetailType::ElectionDropExpired => "election_drop_expired",
            DetailType::ElectionDropOverflow => "election_drop_overflow",
            DetailType::ElectionDropAll => "election_drop_all",
            DetailType::ElectionRestart => "election_restart",
            DetailType::ElectionConfirmed => "election_confirmed",
            DetailType::ElectionNotConfirmed => "election_not_confirmed",
            DetailType::ElectionHintedOverflow => "election_hinted_overflow",
            DetailType::ElectionHintedStarted => "election_hinted_started",
            DetailType::ElectionHintedConfirmed => "election_hinted_confirmed",
            DetailType::ElectionHintedDrop => "election_hinted_drop",
            DetailType::GenerateVote => "generate_vote",
            DetailType::GenerateVoteNormal => "generate_vote_normal",
            DetailType::GenerateVoteFinal => "generate_vote_final",
            DetailType::Blocking => "blocking",
            DetailType::Overflow => "overflow",
            DetailType::TcpAcceptSuccess => "accept_success",
            DetailType::TcpAcceptFailure => "accept_failure",
            DetailType::TcpWriteDrop => "tcp_write_drop",
            DetailType::TcpWriteNoSocketDrop => "tcp_write_no_socket_drop",
            DetailType::TcpExcluded => "tcp_excluded",
            DetailType::TcpMaxPerIp => "tcp_max_per_ip",
            DetailType::TcpMaxPerSubnetwork => "tcp_max_per_subnetwork",
            DetailType::TcpSilentConnectionDrop => "tcp_silent_connection_drop",
            DetailType::TcpIoTimeoutDrop => "tcp_io_timeout_drop",
            DetailType::TcpConnectError => "tcp_connect_error",
            DetailType::TcpReadError => "tcp_read_error",
            DetailType::TcpWriteError => "tcp_write_error",
            DetailType::UnreachableHost => "unreachable_host",
            DetailType::InvalidHeader => "invalid_header",
            DetailType::InvalidMessageType => "invalid_message_type",
            DetailType::InvalidKeepaliveMessage => "invalid_keepalive_message",
            DetailType::InvalidPublishMessage => "invalid_publish_message",
            DetailType::InvalidConfirmReqMessage => "invalid_confirm_req_message",
            DetailType::InvalidConfirmAckMessage => "invalid_confirm_ack_message",
            DetailType::InvalidNodeIdHandshakeMessage => "invalid_node_id_handshake_message",
            DetailType::InvalidTelemetryReqMessage => "invalid_telemetry_req_message",
            DetailType::InvalidTelemetryAckMessage => "invalid_telemetry_ack_message",
            DetailType::InvalidBulkPullMessage => "invalid_bulk_pull_message",
            DetailType::InvalidBulkPullAccountMessage => "invalid_bulk_pull_account_message",
            DetailType::InvalidFrontierReqMessage => "invalid_frontier_req_message",
            DetailType::InvalidAscPullReqMessage => "invalid_asc_pull_req_message",
            DetailType::InvalidAscPullAckMessage => "invalid_asc_pull_ack_message",
            DetailType::MessageTooBig => "message_too_big",
            DetailType::OutdatedVersion => "outdated_version",
            DetailType::UdpMaxPerIp => "udp_max_per_ip",
            DetailType::UdpMaxPerSubnetwork => "udp_max_per_subnetwork",
            DetailType::BlocksConfirmed => "blocks_confirmed",
            DetailType::BlocksConfirmedUnbounded => "blocks_confirmed_unbounded",
            DetailType::BlocksConfirmedBounded => "blocks_confirmed_bounded",
            DetailType::AggregatorAccepted => "aggregator_accepted",
            DetailType::AggregatorDropped => "aggregator_dropped",
            DetailType::RequestsCachedHashes => "requests_cached_hashes",
            DetailType::RequestsGeneratedHashes => "requests_generated_hashes",
            DetailType::RequestsCachedVotes => "requests_cached_votes",
            DetailType::RequestsGeneratedVotes => "requests_generated_votes",
            DetailType::RequestsCachedLateHashes => "requests_cached_late_hashes",
            DetailType::RequestsCachedLateVotes => "requests_cached_late_votes",
            DetailType::RequestsCannotVote => "requests_cannot_vote",
            DetailType::RequestsUnknown => "requests_unknown",
            DetailType::DuplicatePublish => "duplicate_publish",
            DetailType::DifferentGenesisHash => "different_genesis_hash",
            DetailType::InvalidSignature => "invalid_signature",
            DetailType::NodeIdMismatch => "node_id_mismatch",
            DetailType::RequestWithinProtectionCacheZone => "request_within_protection_cache_zone",
            DetailType::NoResponseReceived => "no_response_received",
            DetailType::UnsolicitedTelemetryAck => "unsolicited_telemetry_ack",
            DetailType::FailedSendTelemetryReq => "failed_send_telemetry_req",
            DetailType::GeneratorBroadcasts => "generator_broadcasts",
            DetailType::GeneratorReplies => "generator_replies",
            DetailType::GeneratorRepliesDiscarded => "generator_replies_discarded",
            DetailType::GeneratorSpacing => "generator_spacing",
            DetailType::InvalidNetwork => "invalid_network",
            DetailType::Hinted => "hinted",
            DetailType::InsertFailed => "insert_failed",
            DetailType::MissingBlock => "missing_block",
            DetailType::Response => "response",
            DetailType::WriteDrop => "write_drop",
            DetailType::WriteError => "write_error",
            DetailType::Blocks => "blocks",
            DetailType::Drop => "drop",
            DetailType::BadCount => "bad_count",
            DetailType::ResponseBlocks => "response_blocks",
            DetailType::ResponseAccountInfo => "response_account_info",
            DetailType::ChannelFull => "channel_full",
            DetailType::Activated => "activated",
        }
    }
}

/// Direction of the stat. If the direction is irrelevant, use In
#[derive(FromPrimitive)]
#[repr(u8)]
pub enum Direction {
    In,
    Out,
}

impl Direction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Direction::In => "in",
            Direction::Out => "out",
        }
    }
}

static LOG_COUNT: Lazy<Mutex<Option<FileWriter>>> = Lazy::new(|| Mutex::new(None));
static LOG_SAMPLE: Lazy<Mutex<Option<FileWriter>>> = Lazy::new(|| Mutex::new(None));

pub struct Stat {
    config: StatConfig,
    mutables: Mutex<StatMutables>,
}

impl Stat {
    pub fn new(config: StatConfig) -> Self {
        let default_interval = config.interval;
        let default_capacity = config.capacity;
        Self {
            config,
            mutables: Mutex::new(StatMutables {
                stopped: false,
                entries: HashMap::new(),
                log_last_count_writeout: Instant::now(),
                log_last_sample_writeout: Instant::now(),
                timestamp: Instant::now(),
                default_interval,
                default_capacity,
            }),
        }
    }

    /// Add `value` to stat. If sampling is configured, this will update the current sample.
    ///
    /// # Arguments
    /// * `type` Main statistics type
    /// * `detail` Detail type, or detail::none to register on type-level only
    /// * `dir` Direction
    /// * `value` The amount to add
    /// * `detail_only` If `true`, only update the detail-level counter
    pub fn add(
        &self,
        stat_type: StatType,
        detail: DetailType,
        dir: Direction,
        value: u64,
        detail_only: bool,
    ) -> Result<()> {
        if value == 0 {
            return Ok(());
        }

        const NO_DETAIL_MASK: u32 = 0xffff00ff;
        let key = key_of(stat_type, detail, dir);
        self.update(key, value)?;

        // Optionally update at type-level as well
        if !detail_only && (key & NO_DETAIL_MASK) != key {
            self.update(key & NO_DETAIL_MASK, value)?;
        }

        Ok(())
    }

    pub fn inc(&self, stat_type: StatType, detail: DetailType, dir: Direction) -> Result<()> {
        self.add(stat_type, detail, dir, 1, false)
    }

    pub fn inc_detail_only(
        &self,
        stat_type: StatType,
        detail: DetailType,
        dir: Direction,
    ) -> Result<()> {
        self.add(stat_type, detail, dir, 1, true)
    }

    /// Update count and sample
    ///
    /// # Arguments
    /// * `key` a key constructor from `StatType`, `DetailType` and `Direction`
    /// * `value` Amount to add to the counter
    fn update(&self, key: u32, value: u64) -> Result<()> {
        let now = Instant::now();

        let mut lock = self.mutables.lock().unwrap();
        if !lock.stopped {
            {
                let entry = lock.get_entry_default(key);
                entry.counter.add(value, true);
            }

            let duration = now - lock.log_last_count_writeout;
            if self.config.log_interval_counters > 0
                && duration.as_millis() > self.config.log_interval_counters as u128
            {
                let mut log_count = LOG_COUNT.lock().unwrap();
                let writer = match log_count.as_mut() {
                    Some(x) => x,
                    None => {
                        let writer = FileWriter::new(&self.config.log_counters_filename)?;
                        log_count.get_or_insert(writer)
                    }
                };

                lock.log_counters_impl(writer, &self.config)?;
                lock.log_last_count_writeout = now;
            }

            let entry = lock.get_entry_default(key);
            // Samples
            if self.config.sampling_enabled && entry.sample_interval > 0 {
                entry.sample_current.add(value, false);

                let duration = now - entry.sample_start_time;
                if duration.as_millis() > entry.sample_interval as u128 {
                    entry.sample_start_time = now;

                    // Make a snapshot of samples for thread safety and to get a stable container
                    entry.sample_current.set_timestamp(SystemTime::now());
                    if let Some(samples) = entry.samples.as_mut() {
                        samples.push_back(entry.sample_current.clone());
                    }
                    entry.sample_current.set_value(0);

                    // Log sink
                    let duration = now - lock.log_last_sample_writeout;
                    if self.config.log_interval_samples > 0
                        && duration.as_millis() > self.config.log_interval_samples as u128
                    {
                        let mut log_sample = LOG_SAMPLE.lock().unwrap();
                        let writer = match log_sample.as_mut() {
                            Some(x) => x,
                            None => {
                                let writer = FileWriter::new(&self.config.log_samples_filename)?;
                                log_sample.get_or_insert(writer)
                            }
                        };

                        lock.log_samples_impl(writer, &self.config)?;
                        lock.log_last_sample_writeout = now;
                    }
                }
            }
        }
        Ok(())
    }

    /// Log counters to the given log link
    pub fn log_counters(&self, sink: &mut dyn StatLogSink) -> Result<()> {
        let lock = self.mutables.lock().unwrap();
        lock.log_counters_impl(sink, &self.config)
    }

    /// Log samples to the given log sink
    pub fn log_samples(&self, sink: &mut dyn StatLogSink) -> Result<()> {
        let lock = self.mutables.lock().unwrap();
        lock.log_samples_impl(sink, &self.config)
    }

    /// Define histogram bins. Values are clamped into the first and last bins, but a catch-all bin on one or both
    /// ends can be defined.
    ///
    /// # Examples:
    ///
    ///  // Uniform histogram, total range 12, and 12 bins (each bin has width 1)
    ///  define_histogram (type::vote, detail::confirm_ack, dir::in, {1,13}, 12);
    ///
    ///  // Specific bins matching closed intervals [1,4] [5,19] [20,99]
    ///  define_histogram (type::vote, detail::something, dir::out, {1,5,20,100});
    ///
    ///  // Logarithmic bins matching half-open intervals [1..10) [10..100) [100 1000)
    ///  define_histogram(type::vote, detail::log, dir::out, {1,10,100,1000});
    pub fn define_histogram(
        &self,
        stat_type: StatType,
        detail: DetailType,
        dir: Direction,
        intervals: &[u64],
        bin_count: u64,
    ) {
        let mut lock = self.mutables.lock().unwrap();
        let entry = lock.get_entry_default(key_of(stat_type, detail, dir));
        entry.histogram = Some(StatHistogram::new(intervals, bin_count));
    }

    /// Update histogram
    ///
    /// # Examples:
    ///
    /// // Add 1 to the bin representing a 4-item vbh
    ///  stats.update_histogram(type::vote, detail::confirm_ack, dir::in, 4, 1)
    ///
    ///  // Add 5 to the second bin where 17 falls
    ///  stats.update_histogram(type::vote, detail::something, dir::in, 17, 5)
    ///
    ///  // Add 3 to the last bin as the histogram clamps. You can also add a final bin with maximum end value to effectively prevent this.
    ///  stats.update_histogram(type::vote, detail::log, dir::out, 1001, 3)
    pub fn update_histogram(
        &self,
        stat_type: StatType,
        detail: DetailType,
        dir: Direction,
        index: u64,
        addend: u64,
    ) {
        let mut lock = self.mutables.lock().unwrap();
        let entry = lock.get_entry_default(key_of(stat_type, detail, dir));
        if let Some(histogram) = entry.histogram.as_mut() {
            histogram.add(index, addend);
        }
    }

    pub fn get_histogram(
        &self,
        stat_type: StatType,
        detail: DetailType,
        dir: Direction,
    ) -> Option<StatHistogram> {
        let mut lock = self.mutables.lock().unwrap();
        let entry = lock.get_entry_default(key_of(stat_type, detail, dir));
        entry.histogram.clone()
    }

    /// Returns the duration since `clear()` was last called, or node startup if it's never called.
    pub fn last_reset(&self) -> Duration {
        let lock = self.mutables.lock().unwrap();
        lock.timestamp.elapsed()
    }

    /// Clear all stats
    pub fn clear(&self) {
        let mut lock = self.mutables.lock().unwrap();
        lock.entries.clear();
        lock.timestamp = Instant::now();
    }

    /// Call this to override the default sample interval and capacity, for a specific stat entry.
    /// This must be called before any stat entries are added, as part of the node initialiation.
    pub fn configure(
        &self,
        stat_type: StatType,
        detail: DetailType,
        dir: Direction,
        interval: usize,
        capacity: usize,
    ) {
        self.mutables
            .lock()
            .unwrap()
            .get_entry(key_of(stat_type, detail, dir), interval, capacity);
    }

    /// Disables sampling for a given type/detail/dir combination
    pub fn disable_sampling(&self, stat_type: StatType, detail: DetailType, dir: Direction) {
        self.mutables
            .lock()
            .unwrap()
            .get_entry_default(key_of(stat_type, detail, dir))
            .sample_interval = 0;
    }

    /// Returns current value for the given counter at the type level
    pub fn count(&self, stat_type: StatType, detail: DetailType, dir: Direction) -> u64 {
        self.mutables
            .lock()
            .unwrap()
            .get_entry_default(key_of(stat_type, detail, dir))
            .counter
            .get_value()
    }

    /// Stop stats being output
    pub fn stop(&self) {
        self.mutables.lock().unwrap().stopped = true;
    }
}

/// Constructs a key given type, detail and direction. This is used as input to update(...) and get_entry(...)
fn key_of(stat_type: StatType, detail: DetailType, dir: Direction) -> u32 {
    (stat_type as u32) << 16 | (detail as u32) << 8 | dir as u32
}

pub fn stat_type_as_str(key: u32) -> Result<&'static str> {
    let stat_type: StatType =
        FromPrimitive::from_u32(key >> 16 & 0x000000ff).ok_or_else(|| anyhow!("invalid key"))?;
    Ok(stat_type.as_str())
}

pub fn stat_detail_as_str(key: u32) -> Result<&'static str> {
    let detail: DetailType =
        FromPrimitive::from_u32(key >> 8 & 0x000000ff).ok_or_else(|| anyhow!("invalid key"))?;
    Ok(detail.as_str())
}

pub fn stat_dir_as_str(key: u32) -> Result<&'static str> {
    let stat_dir: Direction =
        FromPrimitive::from_u32(key & 0x000000ff).ok_or_else(|| anyhow!("invalid key"))?;
    Ok(stat_dir.as_str())
}

struct StatMutables {
    /// Whether stats should be output
    stopped: bool,

    /// Stat entries are sorted by key to simplify processing of log output
    entries: HashMap<u32, StatEntry>,

    log_last_count_writeout: Instant,
    log_last_sample_writeout: Instant,

    /// Time of last clear() call
    timestamp: Instant,

    default_interval: usize,
    default_capacity: usize,
}

impl StatMutables {
    fn get_entry_default(&mut self, key: u32) -> &'_ mut StatEntry {
        self.get_entry(key, self.default_interval, self.default_capacity)
    }

    fn get_entry(&mut self, key: u32, interval: usize, capacity: usize) -> &'_ mut StatEntry {
        self.entries
            .entry(key)
            .or_insert_with(|| StatEntry::new(capacity, interval))
    }

    /// Unlocked implementation of log_samples() to avoid using recursive locking
    fn log_samples_impl(&self, sink: &mut dyn StatLogSink, config: &StatConfig) -> Result<()> {
        sink.begin()?;
        if sink.entries() >= config.log_rotation_count {
            sink.rotate()?;
        }

        if config.log_headers {
            let walltime = SystemTime::now();
            sink.write_header("samples", walltime)?;
        }

        for (&key, value) in &self.entries {
            let type_str = stat_type_as_str(key)?;
            let detail = stat_detail_as_str(key)?;
            let dir = stat_dir_as_str(key)?;

            if let Some(samples) = &value.samples {
                for datapoint in samples {
                    let time = datapoint.get_timestamp();
                    sink.write_entry(time, type_str, detail, dir, datapoint.get_value(), None)?;
                }
            }
        }
        sink.inc_entries();
        sink.finalize();
        Ok(())
    }

    /// Unlocked implementation of log_counters() to avoid using recursive locking
    fn log_counters_impl(&self, sink: &mut dyn StatLogSink, config: &StatConfig) -> Result<()> {
        sink.begin()?;
        if sink.entries() >= config.log_rotation_count {
            sink.rotate()?;
        }

        if config.log_headers {
            let walltime = SystemTime::now();
            sink.write_header("counters", walltime)?;
        }

        for (&key, value) in &self.entries {
            let time = value.counter.get_timestamp();
            let type_str = stat_type_as_str(key)?;
            let detail = stat_detail_as_str(key)?;
            let dir = stat_dir_as_str(key)?;
            let histogram = value.histogram.as_ref();
            sink.write_entry(
                time,
                type_str,
                detail,
                dir,
                value.counter.get_value(),
                histogram,
            )?;
        }
        sink.inc_entries();
        sink.finalize();
        Ok(())
    }
}

impl From<MessageType> for DetailType {
    fn from(msg: MessageType) -> Self {
        match msg {
            MessageType::Invalid => DetailType::Invalid,
            MessageType::NotAType => DetailType::NotAType,
            MessageType::Keepalive => DetailType::Keepalive,
            MessageType::Publish => DetailType::Publish,
            MessageType::ConfirmReq => DetailType::ConfirmReq,
            MessageType::ConfirmAck => DetailType::ConfirmAck,
            MessageType::BulkPull => DetailType::BulkPull,
            MessageType::BulkPush => DetailType::BulkPush,
            MessageType::FrontierReq => DetailType::FrontierReq,
            MessageType::NodeIdHandshake => DetailType::NodeIdHandshake,
            MessageType::BulkPullAccount => DetailType::BulkPullAccount,
            MessageType::TelemetryReq => DetailType::TelemetryReq,
            MessageType::TelemetryAck => DetailType::TelemetryAck,
            MessageType::AscPullReq => DetailType::AscPullReq,
            MessageType::AscPullAck => DetailType::AscPullAck,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specific_bins() {
        let stats = Stat::new(StatConfig::new());
        stats.define_histogram(
            StatType::Vote,
            DetailType::ConfirmReq,
            Direction::In,
            &[1, 6, 10, 16],
            0,
        );
        stats.update_histogram(StatType::Vote, DetailType::ConfirmReq, Direction::In, 1, 50);
        let histogram_req = stats
            .get_histogram(StatType::Vote, DetailType::ConfirmReq, Direction::In)
            .unwrap();
        assert_eq!(histogram_req.get_bins()[0].value, 50);
    }

    #[test]
    fn uniform_distribution_and_clamping() {
        // Uniform distribution (12 bins, width 1); also test clamping 100 to the last bin
        let stats = Stat::new(StatConfig::new());
        stats.define_histogram(
            StatType::Vote,
            DetailType::ConfirmAck,
            Direction::In,
            &[1, 13],
            12,
        );
        stats.update_histogram(StatType::Vote, DetailType::ConfirmAck, Direction::In, 1, 1);
        stats.update_histogram(StatType::Vote, DetailType::ConfirmAck, Direction::In, 8, 10);
        stats.update_histogram(
            StatType::Vote,
            DetailType::ConfirmAck,
            Direction::In,
            100,
            1,
        );

        let histogram_ack = stats
            .get_histogram(StatType::Vote, DetailType::ConfirmAck, Direction::In)
            .unwrap();
        assert_eq!(histogram_ack.get_bins()[0].value, 1);
        assert_eq!(histogram_ack.get_bins()[7].value, 10);
        assert_eq!(histogram_ack.get_bins()[11].value, 1);
    }

    #[test]
    fn uniform_distribution() {
        // Uniform distribution (2 bins, width 5); add 1 to each bin
        let stats = Stat::new(StatConfig::new());
        stats.define_histogram(
            StatType::Vote,
            DetailType::ConfirmAck,
            Direction::Out,
            &[1, 11],
            2,
        );
        stats.update_histogram(StatType::Vote, DetailType::ConfirmAck, Direction::Out, 1, 1);
        stats.update_histogram(StatType::Vote, DetailType::ConfirmAck, Direction::Out, 6, 1);

        let histogram_ack_out = stats
            .get_histogram(StatType::Vote, DetailType::ConfirmAck, Direction::Out)
            .unwrap();
        assert_eq!(histogram_ack_out.get_bins()[0].value, 1);
        assert_eq!(histogram_ack_out.get_bins()[1].value, 1);
    }
}
