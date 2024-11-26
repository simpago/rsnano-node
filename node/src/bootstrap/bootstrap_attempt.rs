use super::{bootstrap_limits, BootstrapInitiator, BootstrapMode};
use crate::{
    block_processing::{BlockProcessor, BlockSource},
    utils::HardenedConstants,
    websocket::{OutgoingMessageEnvelope, Topic, WebsocketListener},
};
use anyhow::Result;
use rsnano_core::{encode_hex, utils::PropertyTree, Account, Block};
use rsnano_ledger::Ledger;
use rsnano_network::ChannelId;
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Condvar, Mutex, Weak,
    },
    time::{Duration, Instant},
};
use tracing::debug;

pub trait BootstrapAttemptTrait {
    fn incremental_id(&self) -> u64;
    fn id(&self) -> &str;
    fn started(&self) -> bool;
    fn stopped(&self) -> bool;
    fn stop(&self);
    fn pull_finished(&self);
    fn pulling(&self) -> u32;
    fn total_blocks(&self) -> u64;
    fn inc_total_blocks(&self);
    fn requeued_pulls(&self) -> u32;
    fn inc_requeued_pulls(&self);
    fn pull_started(&self);
    fn duration(&self) -> Duration;
    fn set_started(&self) -> bool;
    fn should_log(&self) -> bool;
    fn notify(&self);
    fn get_information(&self, tree: &mut dyn PropertyTree) -> anyhow::Result<()>;
    fn run(&self);
    fn process_block(
        &self,
        block: Block,
        known_account: &Account,
        pull_blocks_processed: u64,
        max_blocks: u32,
        block_expected: bool,
        retry_limit: u32,
    ) -> bool;
}

pub struct BootstrapAttempt {
    pub incremental_id: u64,
    pub id: String,
    pub mode: BootstrapMode,
    pub total_blocks: AtomicU64,
    next_log: Mutex<Instant>,
    websocket_server: Option<Arc<WebsocketListener>>,
    ledger: Arc<Ledger>,
    attempt_start: Instant,

    /// There is a circular dependency between BlockProcessor and BootstrapAttempt,
    /// that's why we take a Weak reference
    block_processor: Weak<BlockProcessor>,

    /// There is a circular dependency between BootstrapInitiator and BootstrapAttempt,
    /// that's why we take a Weak reference
    pub bootstrap_initiator: Weak<BootstrapInitiator>,
    pub mutex: Mutex<u8>,
    pub condition: Condvar,
    pub pulling: AtomicU32,
    pub requeued_pulls: AtomicU32,
    pub started: AtomicBool,
    pub stopped: AtomicBool,
    pub frontiers_received: AtomicBool,
    bootstrap_started_observer: Arc<Mutex<Vec<Box<dyn Fn(String, String) + Send + Sync>>>>,
    bootstrap_ended_observer:
        Arc<Mutex<Vec<Box<dyn Fn(String, String, String, String) + Send + Sync>>>>,
}

impl BootstrapAttempt {
    pub fn new(
        websocket_server: Option<Arc<WebsocketListener>>,
        block_processor: Weak<BlockProcessor>,
        bootstrap_initiator: Weak<BootstrapInitiator>,
        ledger: Arc<Ledger>,
        id: String,
        mode: BootstrapMode,
        incremental_id: u64,
        bootstrap_started_observer: Arc<Mutex<Vec<Box<dyn Fn(String, String) + Send + Sync>>>>,
        bootstrap_ended_observer: Arc<
            Mutex<Vec<Box<dyn Fn(String, String, String, String) + Send + Sync>>>,
        >,
    ) -> Result<Self> {
        let id = if id.is_empty() {
            encode_hex(HardenedConstants::get().random_128)
        } else {
            id
        };

        let result = Self {
            incremental_id,
            id,
            next_log: Mutex::new(Instant::now()),
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
            bootstrap_started_observer,
            bootstrap_ended_observer,
        };

        result.start()?;
        Ok(result)
    }

    fn start(&self) -> Result<()> {
        let id = &self.id;
        debug!(
            "Starting bootstrap attempt with ID: {id} (mode: {}) ",
            self.mode.as_str()
        );
        if let Some(websocket) = &self.websocket_server {
            websocket.broadcast(&self.bootstrap_started());
        }
        {
            let callbacks = self.bootstrap_started_observer.lock().unwrap();
            for callback in callbacks.iter() {
                (callback)(self.id.clone(), self.mode.as_str().to_string());
            }
        }
        Ok(())
    }

    fn bootstrap_started(&self) -> OutgoingMessageEnvelope {
        OutgoingMessageEnvelope::new(
            Topic::Bootstrap,
            BootstrapStarted {
                reason: "started".to_owned(),
                id: self.id.clone(),
                mode: self.mode.as_str().to_owned(),
            },
        )
    }

    fn bootstrap_exited(&self) -> OutgoingMessageEnvelope {
        OutgoingMessageEnvelope::new(
            Topic::Bootstrap,
            BootstrapExited {
                reason: "exited".to_owned(),
                id: self.id.clone(),
                mode: self.mode.as_str().to_owned(),
                total_blocks: self.total_blocks.load(Ordering::SeqCst).to_string(),
                duration: self.duration().as_secs().to_string(),
            },
        )
    }

    pub fn stop(&self) {
        let lock = self.mutex.lock().unwrap();
        self.stopped.store(true, Ordering::SeqCst);
        drop(lock);
        self.condition.notify_all();
        if let Some(initiator) = self.bootstrap_initiator.upgrade() {
            initiator.clear_pulls(self.incremental_id);
        }
    }

    pub fn should_log(&self) -> bool {
        let mut next_log = self.next_log.lock().unwrap();
        let now = Instant::now();
        if *next_log < now {
            *next_log = now + Duration::from_secs(15);
            true
        } else {
            false
        }
    }

    pub fn process_block(&self, block: Block, pull_blocks_processed: u64) -> bool {
        let mut stop_pull = false;
        let hash = block.hash();
        // If block already exists in the ledger, then we can avoid next part of long account chain
        if pull_blocks_processed % bootstrap_limits::PULL_COUNT_PER_CHECK == 0
            && self
                .ledger
                .any()
                .block_exists_or_pruned(&self.ledger.read_txn(), &hash)
        {
            stop_pull = true;
        } else if let Some(p) = self.block_processor.upgrade() {
            p.add(block, BlockSource::BootstrapLegacy, ChannelId::LOOPBACK);
        }

        stop_pull
    }

    pub fn pull_started(&self) {
        {
            let _lock = self.mutex.lock().unwrap();
            self.pulling.fetch_add(1, Ordering::SeqCst);
        }
        self.condition.notify_all();
    }

    pub fn pull_finished(&self) {
        {
            let _lock = self.mutex.lock().unwrap();
            self.pulling.fetch_sub(1, Ordering::SeqCst);
        }
        self.condition.notify_all();
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    pub fn set_stopped(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    pub fn still_pulling(&self) -> bool {
        let running = !self.stopped.load(Ordering::SeqCst);
        let still_pulling = self.pulling.load(Ordering::SeqCst) > 0;
        running && still_pulling
    }

    pub fn duration(&self) -> Duration {
        self.attempt_start.elapsed()
    }

    pub fn add_bootstrap_started_callback(&self, f: Box<dyn Fn(String, String) + Send + Sync>) {
        self.bootstrap_started_observer.lock().unwrap().push(f);
    }

    pub fn add_bootstrap_ended_callback(
        &self,
        f: Box<dyn Fn(String, String, String, String) + Send + Sync>,
    ) {
        self.bootstrap_ended_observer.lock().unwrap().push(f);
    }
}

impl Drop for BootstrapAttempt {
    fn drop(&mut self) {
        let id = &self.id;
        debug!(
            "Exiting bootstrap attempt with ID: {id} (mode: {})",
            self.mode.as_str()
        );

        if let Some(websocket) = &self.websocket_server {
            websocket.broadcast(&self.bootstrap_exited());
        }

        {
            let callbacks = self.bootstrap_ended_observer.lock().unwrap();
            for callback in callbacks.iter() {
                (callback)(
                    self.id.clone(),
                    self.mode.as_str().to_string(),
                    self.total_blocks.load(Ordering::SeqCst).to_string(),
                    self.duration().as_secs().to_string(),
                )
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BootstrapStarted {
    pub reason: String,
    pub id: String,
    pub mode: String,
}

#[derive(Serialize, Deserialize)]
pub struct BootstrapExited {
    pub reason: String,
    pub id: String,
    pub mode: String,
    pub total_blocks: String,
    pub duration: String,
}
