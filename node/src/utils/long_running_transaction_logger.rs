use backtrace::Backtrace;
use rsnano_store_lmdb::TransactionTracker;
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use tracing::warn;

#[derive(Clone, Debug, PartialEq)]
pub struct TxnTrackingConfig {
    /** If true, enable tracking for transaction read/writes held open longer than the min time variables */
    pub enable: bool,
    pub min_read_txn_time_ms: i64,
    pub min_write_txn_time_ms: i64,
    pub ignore_writes_below_block_processor_max_time: bool,
}

impl TxnTrackingConfig {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for TxnTrackingConfig {
    fn default() -> Self {
        Self {
            enable: false,
            min_read_txn_time_ms: 5000,
            min_write_txn_time_ms: 500,
            ignore_writes_below_block_processor_max_time: true,
        }
    }
}

pub struct LongRunningTransactionLogger {
    stats: Mutex<HashMap<u64, TxnStats>>,
    config: TxnTrackingConfig,
    block_processor_batch_max_time: Duration,
}

impl LongRunningTransactionLogger {
    pub fn new(config: TxnTrackingConfig, block_processor_batch_max_time: Duration) -> Self {
        Self {
            config,
            block_processor_batch_max_time,
            stats: Mutex::new(HashMap::new()),
        }
    }

    pub fn add(&self, txn_id: u64, is_write: bool) {
        let mut stats = self.stats.lock().unwrap();
        stats.insert(
            txn_id,
            TxnStats {
                is_write,
                start: Instant::now(),
                thread_name: std::thread::current().name().map(|s| s.to_owned()),
                stacktrace: Backtrace::new_unresolved(),
            },
        );
    }

    pub fn erase(&self, txn_id: u64, _is_write: bool) {
        let entry = {
            let mut stats = self.stats.lock().unwrap();
            stats.remove(&txn_id)
        };

        if let Some(mut entry) = entry {
            self.log_if_held_long_enough(&mut entry);
        }
    }

    fn log_if_held_long_enough(&self, txn: &mut TxnStats) {
        // Only log these transactions if they were held for longer than the min_read_txn_time/min_write_txn_time config values
        let time_open = txn.start.elapsed();
        // Reduce noise in log files by removing any entries from the block processor (if enabled) which are less than the max batch time (+ a few second buffer) because these are expected writes during bootstrapping.
        let is_below_max_time =
            time_open <= (self.block_processor_batch_max_time + Duration::from_secs(3));
        let is_blk_processing_thread = txn.thread_name.as_deref() == Some("Blck processing");
        if self.config.ignore_writes_below_block_processor_max_time
            && is_blk_processing_thread
            && txn.is_write
            && is_below_max_time
        {
            return;
        }

        if (txn.is_write
            && time_open >= Duration::from_millis(self.config.min_write_txn_time_ms as u64))
            || (!txn.is_write
                && time_open >= Duration::from_millis(self.config.min_read_txn_time_ms as u64))
        {
            let txn_type = if txn.is_write { "write lock" } else { "read" };
            txn.stacktrace.resolve();
            warn!(
                "{}ms {} held on thread {}\n{:?}",
                time_open.as_millis(),
                txn_type,
                txn.thread_name.as_deref().unwrap_or("unnamed"),
                txn.stacktrace
            );
        }
    }
}

#[derive(Clone)]
struct TxnStats {
    is_write: bool,
    thread_name: Option<String>,
    start: Instant,
    stacktrace: Backtrace,
}

impl TransactionTracker for LongRunningTransactionLogger {
    fn txn_start(&self, txn_id: u64, is_write: bool) {
        self.add(txn_id, is_write);
    }

    fn txn_end(&self, txn_id: u64, is_write: bool) {
        self.erase(txn_id, is_write);
    }
}
