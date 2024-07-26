use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use super::{
    BootstrapClient, BootstrapConnections, BootstrapConnectionsExt, BootstrapInitiator,
    BootstrapStrategy, PullInfo,
};
use crate::{
    block_processing::{BlockProcessor, BlockSource},
    bootstrap::BootstrapMode,
    stats::{DetailType, Direction, StatType, Stats},
    transport::{BlockDeserializer, BufferDropPolicy, TrafficType},
    utils::{AsyncRuntime, ErrorCode, ThreadPool},
};
use rsnano_core::{work::WorkThresholds, Account, BlockEnum, BlockHash};
use rsnano_messages::{BulkPull, Message};
use tracing::{debug, trace};

pub struct BulkPullClient {
    /// Tracks the next block expected to be received starting with the block hash that was expected and followed by previous blocks for this account chain
    expected: Mutex<BlockHash>,
    /// Original pull request
    pull: PullInfo,
    connection: Arc<BootstrapClient>,
    attempt: Arc<BootstrapStrategy>,
    stats: Arc<Stats>,
    network_error: AtomicBool,
    block_processor: Arc<BlockProcessor>,
    workers: Arc<dyn ThreadPool>,
    block_deserializer: BlockDeserializer,
    /// Tracks the number of blocks successfully deserialized
    pull_blocks: AtomicU64,
    connections: Arc<BootstrapConnections>,
    config: BulkPullClientConfig,
    /// Tracks the number of times an unexpected block was received
    unexpected_count: AtomicU64,

    /// Tracks the account number for this account chain
    /// Used when an account chain has a mix between state blocks and legacy blocks which do not encode the account number in the block
    /// 0 if the account is unknown
    known_account: Mutex<Account>,
    bootstrap_initiator: Arc<BootstrapInitiator>,
}

pub struct BulkPullClientConfig {
    pub disable_legacy_bootstrap: bool,
    pub retry_limit: u32,
    pub work_thresholds: WorkThresholds,
}

impl BulkPullClient {
    pub fn new(
        config: BulkPullClientConfig,
        stats: Arc<Stats>,
        block_processor: Arc<BlockProcessor>,
        connection: Arc<BootstrapClient>,
        attempt: Arc<BootstrapStrategy>,
        workers: Arc<dyn ThreadPool>,
        async_rt: Arc<AsyncRuntime>,
        connections: Arc<BootstrapConnections>,
        bootstrap_initiator: Arc<BootstrapInitiator>,
        pull: PullInfo,
    ) -> Self {
        let result = Self {
            expected: Mutex::new(BlockHash::zero()),
            pull,
            connection,
            attempt,
            stats,
            network_error: AtomicBool::new(false),
            block_processor,
            workers,
            block_deserializer: BlockDeserializer::new(async_rt),
            pull_blocks: AtomicU64::new(0),
            connections,
            config,
            unexpected_count: AtomicU64::new(0),
            known_account: Mutex::new(Account::zero()),
            bootstrap_initiator,
        };
        result.attempt.attempt().condition.notify_all();
        result
    }
}

impl Drop for BulkPullClient {
    fn drop(&mut self) {
        /* If received end block is not expected end block
        Or if given start and end blocks are from different chains (i.e. forked node or malicious node) */
        let expected = self.expected.lock().unwrap();
        if *expected != self.pull.end && !expected.is_zero() {
            self.pull.head = *expected;
            if self.attempt.mode() != BootstrapMode::Legacy {
                self.pull.account_or_head = expected.clone().into();
            }
            self.pull.processed += self.pull_blocks.load(Ordering::SeqCst)
                - self.unexpected_count.load(Ordering::SeqCst);
            self.connections
                .requeue_pull(self.pull.clone(), self.network_error.load(Ordering::SeqCst));

            debug!(
                "Bulk pull end block is not expected {} for account {} or head block {}",
                self.pull.end,
                Account::from(self.pull.account_or_head).encode_account(),
                self.pull.account_or_head
            );
        } else {
            self.bootstrap_initiator.remove_from_cache(&self.pull);
        }
        self.attempt.attempt().pull_finished();
    }
}

pub trait BulkPullClientExt {
    fn request(&self);
    fn throttled_receive_block(&self);
    fn receive_block(&self);
    fn received_block(&self, ec: ErrorCode, block: Option<BlockEnum>);
}

impl BulkPullClientExt for Arc<BulkPullClient> {
    fn request(&self) {
        debug_assert!(
            !self.pull.head.is_zero() || self.pull.retry_limit <= self.config.retry_limit
        );
        *self.expected.lock().unwrap() = self.pull.head;
        let mut payload = BulkPull::default();
        if self.pull.head == self.pull.head_original && self.pull.attempts % 4 < 3 {
            // Account for new pulls
            payload.start = self.pull.account_or_head;
        } else {
            // Head for cached pulls or accounts with public key equal to existing block hash (25% of attempts)
            payload.start = self.pull.account_or_head;
        }
        payload.end = self.pull.end;
        payload.count = self.pull.count;
        payload.ascending = false;

        trace!(
            account_or_head = %self.pull.account_or_head,
            channel = self.connection.channel_string(),
            "Requesting account or head"
        );

        if self.attempt.attempt().should_log() {
            debug!(
                "Accounts in pull queue: {}",
                self.attempt.attempt().pulling.load(Ordering::Relaxed)
            );
        }

        let self_clone = Arc::clone(self);
        self.connection.send(
            &Message::BulkPull(payload),
            Some(Box::new(move |ec, _len| {
                if ec.is_ok() {
                    self_clone.throttled_receive_block();
                } else {
                    debug!(
                        "Error sending bulk pull request to: {} ({:?})",
                        self_clone.connection.channel_string(),
                        ec
                    );
                    self_clone.stats.inc_dir(
                        StatType::Bootstrap,
                        DetailType::BulkPullRequestFailure,
                        Direction::In,
                    );
                }
            })),
            BufferDropPolicy::NoLimiterDrop,
            TrafficType::Generic,
        );
    }

    fn throttled_receive_block(&self) {
        debug_assert!(!self.network_error.load(Ordering::Relaxed));
        if self.block_processor.queue_len(BlockSource::BootstrapLegacy) < 1024 {
            self.receive_block();
        } else {
            let self_clone = Arc::clone(self);
            self.workers.add_delayed_task(
                Duration::from_secs(1),
                Box::new(move || {
                    if !self_clone.connection.pending_stop() && !self_clone.attempt.stopped() {
                        self_clone.throttled_receive_block();
                    }
                }),
            );
        }
    }

    fn receive_block(&self) {
        let socket = self.connection.get_socket();
        let self_clone = Arc::clone(self);
        self.block_deserializer.read(
            socket,
            Box::new(move |ec, block| {
                self_clone.received_block(ec, block);
            }),
        );
    }

    fn received_block(&self, ec: ErrorCode, block: Option<BlockEnum>) {
        if ec.is_err() {
            self.network_error.store(true, Ordering::SeqCst);
            return;
        }
        let Some(block) = block else {
            // Avoid re-using slow peers, or peers that sent the wrong blocks.
            if !self.connection.pending_stop()
                && (*self.expected.lock().unwrap() == self.pull.end
                    || (self.pull.count != 0
                        && self.pull.count as u64 == self.pull_blocks.load(Ordering::SeqCst)))
            {
                self.connections
                    .pool_connection(Arc::clone(&self.connection), false, false);
            }
            return;
        };

        if self.config.work_thresholds.validate_entry_block(&block) {
            debug!("Insufficient work for bulk pull block: {}", block.hash());
            self.stats
                .inc(StatType::Error, DetailType::InsufficientWork);
            return;
        }
        let hash = block.hash();
        trace!(block = block.to_json().unwrap(), "Pulled block");

        // Is block expected?
        let mut block_expected = false;
        let expected = self.expected.lock().unwrap().clone();
        // Unconfirmed head is used only for lazy destinations if legacy bootstrap is not available, see nano::bootstrap_attempt::lazy_destinations_increment (...)
        let unconfirmed_account_head = self.config.disable_legacy_bootstrap
            && self.pull_blocks.load(Ordering::SeqCst) == 0
            && self.pull.retry_limit <= self.config.retry_limit
            && expected == self.pull.account_or_head.into()
            && block.account_field() == Some(self.pull.account_or_head.into());

        if hash == expected || unconfirmed_account_head {
            *self.expected.lock().unwrap() = block.previous();
            block_expected = true;
        } else {
            self.unexpected_count.fetch_add(1, Ordering::SeqCst);
        }

        if self.pull_blocks.load(Ordering::SeqCst) == 0 && block_expected {
            *self.known_account.lock().unwrap() = block.account_field().unwrap_or_default();
        }

        if self.connection.inc_block_count() == 0 {
            self.connection.set_start_time();
        }

        self.attempt
            .attempt()
            .total_blocks
            .fetch_add(1, Ordering::SeqCst);

        self.pull_blocks.fetch_add(1, Ordering::SeqCst);
        let block = Arc::new(block);

        let stop_pull = self.attempt.process_block(
            block,
            &self.known_account.lock().unwrap(),
            self.pull_blocks.load(Ordering::SeqCst),
            self.pull.count,
            block_expected,
            self.pull.retry_limit,
        );

        if !stop_pull && !self.connection.hard_stop() {
            /* Process block in lazy pull if not stopped
            Stop usual pull request with unexpected block & more than 16k blocks processed
            to prevent spam */
            if self.attempt.mode() != BootstrapMode::Legacy
                || self.unexpected_count.load(Ordering::SeqCst) < 16384
            {
                self.throttled_receive_block();
            }
        } else if !stop_pull && block_expected {
            self.connections
                .pool_connection(Arc::clone(&self.connection), false, false);
        }
    }
}
