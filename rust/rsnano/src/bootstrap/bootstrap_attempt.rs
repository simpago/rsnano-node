use crate::{
    block_processor::BlockProcessor,
    datastore::Ledger,
    encode_hex,
    logger_mt::Logger,
    unchecked_info::UncheckedInfo,
    websocket::{Listener, MessageBuilder},
    Account, BlockEnum, HardenedConstants,
};
use anyhow::Result;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Condvar, Mutex, RwLock, Weak,
    },
    time::{Duration, Instant},
};

use super::{bootstrap_limits, BootstrapInitiator, BootstrapMode};

pub struct BootstrapAttempt {
    pub incremental_id: u64,
    pub id: String,
    pub mode: BootstrapMode,
    pub total_blocks: AtomicU64,
    next_log: Mutex<Instant>,
    logger: Arc<dyn Logger>,
    websocket_server: Arc<dyn Listener>,
    ledger: Arc<Ledger>,
    attempt_start: Instant,

    /// There is a circular dependency between BlockProcessor and BootstrapAttempt,
    /// that's why we take a Weak reference
    block_processor: Weak<BlockProcessor>,

    /// There is a circular dependency between BootstrapInitiator and BootstrapAttempt,
    /// that's why we take a Weak reference
    bootstrap_initiator: Weak<BootstrapInitiator>,
    pub mutex: Mutex<u8>,
    pub condition: Condvar,
    pub pulling: AtomicU32,
    pub requeued_pulls: AtomicU32,
    pub started: AtomicBool,
    pub stopped: AtomicBool,
    pub frontiers_received: AtomicBool,
}

impl BootstrapAttempt {
    pub(crate) fn new(
        logger: Arc<dyn Logger>,
        websocket_server: Arc<dyn Listener>,
        block_processor: Weak<BlockProcessor>,
        bootstrap_initiator: Weak<BootstrapInitiator>,
        ledger: Arc<Ledger>,
        id: &str,
        mode: BootstrapMode,
        incremental_id: u64,
    ) -> Result<Self> {
        let id = if id.is_empty() {
            encode_hex(HardenedConstants::get().random_128)
        } else {
            id.to_owned()
        };

        let result = Self {
            incremental_id,
            id,
            next_log: Mutex::new(Instant::now()),
            logger,
            block_processor,
            bootstrap_initiator,
            mode,
            websocket_server,
            ledger,
            attempt_start: Instant::now(),
            total_blocks: AtomicU64::new(0),
            mutex: Mutex::new(0),
            condition: Condvar::new(),
            pulling: AtomicU32::new(0),
            started: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            requeued_pulls: AtomicU32::new(0),
            frontiers_received: AtomicBool::new(false),
        };

        result.start()?;
        Ok(result)
    }

    fn start(&self) -> Result<()> {
        let mode = self.mode_text();
        let id = &self.id;
        self.logger
            .always_log(&format!("Starting {mode} bootstrap attempt with ID {id}"));
        self.websocket_server
            .broadcast(&MessageBuilder::bootstrap_started(id, mode)?)?;
        Ok(())
    }

    pub(crate) fn stop(&self) {
        let lock = self.mutex.lock().unwrap();
        self.stopped.store(true, Ordering::SeqCst);
        drop(lock);
        self.condition.notify_all();
        if let Some(initiator) = self.bootstrap_initiator.upgrade() {
            initiator.clear_pulls(self.incremental_id);
        }
    }

    pub(crate) fn should_log(&self) -> bool {
        let mut next_log = self.next_log.lock().unwrap();
        let now = Instant::now();
        if *next_log < now {
            *next_log = now + Duration::from_secs(15);
            true
        } else {
            false
        }
    }

    pub(crate) fn mode_text(&self) -> &'static str {
        match self.mode {
            BootstrapMode::Legacy => "legacy",
            BootstrapMode::Lazy => "lazy",
            BootstrapMode::WalletLazy => "wallet_lazy",
        }
    }

    pub(crate) fn process_block(
        &self,
        block: Arc<RwLock<BlockEnum>>,
        known_account: &Account,
        pull_blocks_processed: u64,
        _max_blocks: u32,
        _block_expected: bool,
        _retry_limit: u32,
    ) -> bool {
        let mut stop_pull = false;
        let hash = { block.read().unwrap().as_block().hash() };
        // If block already exists in the ledger, then we can avoid next part of long account chain
        if pull_blocks_processed % bootstrap_limits::PULL_COUNT_PER_CHECK == 0
            && self.ledger.block_or_pruned_exists(&hash)
        {
            stop_pull = true;
        } else {
            let unchecked_info =
                UncheckedInfo::new(block, known_account, crate::SignatureVerification::Unknown);
            if let Some(p) = self.block_processor.upgrade() {
                p.add(&unchecked_info);
            }
        }

        stop_pull
    }

    pub(crate) fn pull_started(&self) {
        {
            let _lock = self.mutex.lock().unwrap();
            self.pulling.fetch_add(1, Ordering::SeqCst);
        }
        self.condition.notify_all();
    }

    pub(crate) fn pull_finished(&self) {
        {
            let _lock = self.mutex.lock().unwrap();
            self.pulling.fetch_sub(1, Ordering::SeqCst);
        }
        self.condition.notify_all();
    }

    pub(crate) fn stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    pub(crate) fn set_stopped(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    pub(crate) fn still_pulling(&self) -> bool {
        debug_assert!(self.mutex.try_lock().is_err());
        let running = !self.stopped.load(Ordering::SeqCst);
        let still_pulling = self.pulling.load(Ordering::SeqCst) > 0;
        running && still_pulling
    }

    pub(crate) fn duration(&self) -> Duration {
        self.attempt_start.elapsed()
    }
}

impl Drop for BootstrapAttempt {
    fn drop(&mut self) {
        let mode = self.mode_text();
        let id = &self.id;
        self.logger
            .always_log(&format!("Exiting {mode} bootstrap attempt with ID {id}"));

        self.websocket_server
            .broadcast(
                &MessageBuilder::bootstrap_exited(
                    id,
                    mode,
                    self.duration(),
                    self.total_blocks.load(Ordering::SeqCst),
                )
                .unwrap(),
            )
            .unwrap();
    }
}
