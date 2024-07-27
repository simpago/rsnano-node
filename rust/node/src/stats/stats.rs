use super::{DetailType, Direction, Sample, StatType, StatsJsonWriter};
use super::{StatFileWriter, StatsLogSink};
use crate::config::StatsConfig;
use anyhow::Result;
use bounded_vec_deque::BoundedVecDeque;
use once_cell::sync::Lazy;
use rsnano_core::utils::get_env_bool;
use rsnano_messages::MessageType;
use std::{
    collections::BTreeMap,
    sync::{atomic::AtomicU64, Arc, Condvar, Mutex, RwLock},
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime},
};
use tracing::debug;

pub struct Stats {
    config: StatsConfig,
    mutables: Arc<RwLock<StatMutables>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    stats_loop: Arc<StatsLoop>,
    enable_logging: bool,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new(StatsConfig::default())
    }
}

impl Stats {
    pub fn new(config: StatsConfig) -> Self {
        let mutables = Arc::new(RwLock::new(StatMutables {
            counters: BTreeMap::new(),
            samplers: BTreeMap::new(),
            timestamp: Instant::now(),
        }));
        Self {
            config: config.clone(),
            thread: Mutex::new(None),
            stats_loop: Arc::new(StatsLoop {
                condition: Condvar::new(),
                mutables: Arc::clone(&mutables),
                config,
                loop_state: Mutex::new(StatsLoopState {
                    stopped: false,
                    log_last_count_writeout: Instant::now(),
                    log_last_sample_writeout: Instant::now(),
                }),
            }),
            mutables,
            enable_logging: get_env_bool("NANO_LOG_STATS").unwrap_or(false),
        }
    }

    pub fn start(&self) {
        if !self.should_run() {
            return;
        };

        let stats_loop = Arc::clone(&self.stats_loop);
        *self.thread.lock().unwrap() = Some(
            std::thread::Builder::new()
                .name("Stats".to_string())
                .spawn(move || stats_loop.run())
                .unwrap(),
        );
    }

    fn should_run(&self) -> bool {
        !self.config.log_counters_interval.is_zero() || !self.config.log_samples_interval.is_zero()
    }

    /// Stop stats being output
    pub fn stop(&self) {
        self.stats_loop.loop_state.lock().unwrap().stopped = true;
        self.stats_loop.condition.notify_all();
        if let Some(handle) = self.thread.lock().unwrap().take() {
            handle.join().unwrap();
        }
    }

    /// Add `value` to given counter
    pub fn add(&self, stat_type: StatType, detail: DetailType, value: u64) {
        self.add_dir(stat_type, detail, Direction::In, value)
    }

    /// Add `value` to given counter
    pub fn add_dir(&self, stat_type: StatType, detail: DetailType, dir: Direction, value: u64) {
        if value == 0 {
            return;
        }

        self.log_add(stat_type, detail, dir, value);

        let key = CounterKey::new(stat_type, detail, dir);

        // This is a two-step process to avoid exclusively locking the mutex in the common case
        {
            let lock = self.mutables.read().unwrap();

            if let Some(counter) = lock.counters.get(&key) {
                counter.add(value);
                return;
            }
        }
        // Not found, create a new entry
        {
            let mut lock = self.mutables.write().unwrap();
            let counter = lock.counters.entry(key).or_insert(CounterEntry::new());
            counter.add(value);

            let all_key = CounterKey::new(stat_type, DetailType::All, dir);
            if key != all_key {
                lock.counters.entry(all_key).or_insert(CounterEntry::new());
            }
        }
    }

    fn log_add(&self, stat_type: StatType, detail: DetailType, dir: Direction, value: u64) {
        if self.enable_logging {
            debug!(
                "Stat: {:?}::{:?}::{:?} += {}",
                stat_type, detail, dir, value
            );
        }
    }

    pub fn add_dir_aggregate(
        &self,
        stat_type: StatType,
        detail: DetailType,
        dir: Direction,
        value: u64,
    ) {
        if value == 0 {
            return;
        }

        self.log_add(stat_type, detail, dir, value);

        let key = CounterKey::new(stat_type, detail, dir);
        let all_key = CounterKey::new(stat_type, DetailType::All, dir);

        // This is a two-step process to avoid exclusively locking the mutex in the common case
        {
            let lock = self.mutables.read().unwrap();

            if let Some(counter) = lock.counters.get(&key) {
                counter.add(value);
                if key != all_key {
                    let all_counter = lock.counters.get(&all_key).unwrap();
                    all_counter.add(value);
                }
                return;
            }
        }
        // Not found, create a new entry
        {
            let mut lock = self.mutables.write().unwrap();
            let counter = lock.counters.entry(key).or_insert(CounterEntry::new());
            counter.add(value);
            if key != all_key {
                let all_counter = lock.counters.entry(all_key).or_insert(CounterEntry::new());
                all_counter.add(value);
            }
        }
    }

    pub fn inc(&self, stat_type: StatType, detail: DetailType) {
        self.add_dir(stat_type, detail, Direction::In, 1)
    }

    pub fn inc_dir(&self, stat_type: StatType, detail: DetailType, dir: Direction) {
        self.add_dir(stat_type, detail, dir, 1)
    }

    pub fn inc_dir_aggregate(&self, stat_type: StatType, detail: DetailType, dir: Direction) {
        self.add_dir_aggregate(stat_type, detail, dir, 1)
    }

    pub fn sample(&self, sample: Sample, value: i64, expected_min_max: (i64, i64)) {
        self.log_sample(sample, value);
        let key = SamplerKey::new(sample);
        // This is a two-step process to avoid exclusively locking the mutex in the common case
        {
            let lock = self.mutables.read().unwrap();
            if let Some(sampler) = lock.samplers.get(&key) {
                sampler.add(value);
                return;
            }
        }
        // Not found, create a new entry
        {
            let mut lock = self.mutables.write().unwrap();
            let sampler = lock
                .samplers
                .entry(key)
                .or_insert(SamplerEntry::new(self.config.max_samples, expected_min_max));
            sampler.add(value)
        }
    }

    fn log_sample(&self, sample: Sample, value: i64) {
        if self.enable_logging {
            debug!("Sample: {:?} -> {}", sample, value);
        }
    }

    pub fn samples(&self, sample: Sample) -> Vec<i64> {
        let key = SamplerKey::new(sample);
        let lock = self.mutables.read().unwrap();
        if let Some(sampler) = lock.samplers.get(&key) {
            sampler.collect()
        } else {
            Vec::new()
        }
    }

    /// Log counters to the given log link
    pub fn log_counters(&self, sink: &mut dyn StatsLogSink) -> Result<()> {
        let now = SystemTime::now();
        let lock = self.mutables.write().unwrap();
        lock.log_counters_impl(sink, &self.config, now)
    }

    /// Log samples to the given log sink
    pub fn log_samples(&self, sink: &mut dyn StatsLogSink) -> Result<()> {
        let now = SystemTime::now();
        let lock = self.mutables.write().unwrap();
        lock.log_samples_impl(sink, &self.config, now)
    }

    /// Returns the duration since `clear()` was last called, or node startup if it's never called.
    pub fn last_reset(&self) -> Duration {
        let lock = self.mutables.read().unwrap();
        lock.timestamp.elapsed()
    }

    /// Clear all stats
    pub fn clear(&self) {
        let mut lock = self.mutables.write().unwrap();
        lock.counters.clear();
        lock.samplers.clear();
        lock.timestamp = Instant::now();
    }
    ///
    /// Returns current value for the given counter at the type level
    pub fn count_all(&self, stat_type: StatType, dir: Direction) -> u64 {
        let guard = self.mutables.read().unwrap();
        let start = CounterKey::new(stat_type, DetailType::All, dir);
        let mut result = 0u64;
        for (key, entry) in guard.counters.range(start..) {
            if key.stat_type != stat_type {
                break;
            }
            if key.dir == dir && key.detail != DetailType::All {
                result += u64::from(entry);
            }
        }
        result
    }

    /// Returns current value for the given counter at the type level
    pub fn count(&self, stat_type: StatType, detail: DetailType, dir: Direction) -> u64 {
        let key = CounterKey::new(stat_type, detail, dir);
        self.mutables
            .read()
            .unwrap()
            .counters
            .get(&key)
            .map(|i| i.into())
            .unwrap_or_default()
    }

    pub fn dump(&self, category: StatCategory) -> String {
        let mut sink = StatsJsonWriter::new();
        match category {
            StatCategory::Counters => self.log_counters(&mut sink).unwrap(),
            StatCategory::Samples => self.log_samples(&mut sink).unwrap(),
        }
        sink.to_string()
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct CounterKey {
    stat_type: StatType,
    detail: DetailType,
    dir: Direction,
}

impl CounterKey {
    fn new(stat_type: StatType, detail: DetailType, dir: Direction) -> Self {
        Self {
            stat_type,
            detail,
            dir,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct SamplerKey {
    sample: Sample,
}

impl SamplerKey {
    fn new(sample: Sample) -> Self {
        Self { sample }
    }
}

pub enum StatCategory {
    Counters,
    Samples,
}

struct StatMutables {
    /// Stat entries are sorted by key to simplify processing of log output
    counters: BTreeMap<CounterKey, CounterEntry>,
    samplers: BTreeMap<SamplerKey, SamplerEntry>,

    /// Time of last clear() call
    timestamp: Instant,
}

impl StatMutables {
    /// Unlocked implementation of log_samples() to avoid using recursive locking
    fn log_samples_impl(
        &self,
        sink: &mut dyn StatsLogSink,
        config: &StatsConfig,
        time: SystemTime,
    ) -> Result<()> {
        sink.begin()?;
        if sink.entries() >= config.log_rotation_count {
            sink.rotate()?;
        }

        if config.log_headers {
            let walltime = SystemTime::now();
            sink.write_header("samples", walltime)?;
        }

        for (&key, entry) in &self.samplers {
            let sample = key.sample.as_str();
            sink.write_sampler_entry(time, sample, entry.collect(), entry.expected_min_max)?;
        }
        sink.inc_entries();
        sink.finalize();
        Ok(())
    }

    /// Unlocked implementation of log_counters() to avoid using recursive locking
    fn log_counters_impl(
        &self,
        sink: &mut dyn StatsLogSink,
        config: &StatsConfig,
        time: SystemTime,
    ) -> Result<()> {
        sink.begin()?;
        if sink.entries() >= config.log_rotation_count {
            sink.rotate()?;
        }

        if config.log_headers {
            let walltime = SystemTime::now();
            sink.write_header("counters", walltime)?;
        }

        for (&key, entry) in &self.counters {
            let type_str = key.stat_type.as_str();
            let detail = key.detail.as_str();
            let dir = key.dir.as_str();
            sink.write_counter_entry(time, type_str, detail, dir, entry.into())?;
        }
        sink.inc_entries();
        sink.finalize();
        Ok(())
    }
}

struct CounterEntry(AtomicU64);

impl CounterEntry {
    fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    fn add(&self, value: u64) {
        self.0.fetch_add(value, std::sync::atomic::Ordering::SeqCst);
    }
}

impl From<&CounterEntry> for u64 {
    fn from(value: &CounterEntry) -> Self {
        value.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

struct SamplerEntry {
    samples: Mutex<BoundedVecDeque<i64>>,
    pub expected_min_max: (i64, i64),
}

impl SamplerEntry {
    pub fn new(max_samples: usize, expected_min_max: (i64, i64)) -> Self {
        Self {
            samples: Mutex::new(BoundedVecDeque::new(max_samples)),
            expected_min_max,
        }
    }

    fn add(&self, value: i64) {
        self.samples.lock().unwrap().push_back(value);
    }

    fn collect(&self) -> Vec<i64> {
        let mut guard = self.samples.lock().unwrap();
        guard.drain(..).collect()
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

struct StatsLoop {
    mutables: Arc<RwLock<StatMutables>>,
    condition: Condvar,
    loop_state: Mutex<StatsLoopState>,
    config: StatsConfig,
}

impl StatsLoop {
    fn run(&self) {
        let mut guard = self.loop_state.lock().unwrap();
        while !guard.stopped {
            guard = self
                .condition
                .wait_timeout_while(guard, Duration::from_secs(1), |g| !g.stopped)
                .unwrap()
                .0;

            if !guard.stopped {
                self.run_one(&mut guard).unwrap();
            }
        }
    }

    fn run_one(&self, lock: &mut StatsLoopState) -> anyhow::Result<()> {
        let stats = self.mutables.read().unwrap();
        // Counters
        if !self.config.log_counters_interval.is_zero()
            && lock.log_last_count_writeout.elapsed() > self.config.log_counters_interval
        {
            let mut log_count = LOG_COUNT.lock().unwrap();
            let writer = match log_count.as_mut() {
                Some(x) => x,
                None => {
                    let writer = StatFileWriter::new(&self.config.log_counters_filename)?;
                    log_count.get_or_insert(writer)
                }
            };

            stats.log_counters_impl(writer, &self.config, SystemTime::now())?;
            lock.log_last_count_writeout = Instant::now();
        }

        // Samples
        if !self.config.log_samples_interval.is_zero()
            && lock.log_last_sample_writeout.elapsed() > self.config.log_samples_interval
        {
            let mut log_sample = LOG_SAMPLE.lock().unwrap();
            let writer = match log_sample.as_mut() {
                Some(x) => x,
                None => {
                    let writer = StatFileWriter::new(&self.config.log_samples_filename)?;
                    log_sample.get_or_insert(writer)
                }
            };
            stats.log_samples_impl(writer, &self.config, SystemTime::now())?;
            lock.log_last_sample_writeout = Instant::now();
        }

        Ok(())
    }
}

struct StatsLoopState {
    stopped: bool,
    log_last_count_writeout: Instant,
    log_last_sample_writeout: Instant,
}

static LOG_COUNT: Lazy<Mutex<Option<StatFileWriter>>> = Lazy::new(|| Mutex::new(None));
static LOG_SAMPLE: Lazy<Mutex<Option<StatFileWriter>>> = Lazy::new(|| Mutex::new(None));

#[cfg(test)]
mod tests {
    use super::*;

    /// Test stat counting at both type and detail levels
    #[test]
    fn counters() {
        let stats = Stats::new(StatsConfig::new());
        stats.add_dir_aggregate(StatType::Ledger, DetailType::All, Direction::In, 1);
        stats.add_dir_aggregate(StatType::Ledger, DetailType::All, Direction::In, 5);
        stats.inc_dir_aggregate(StatType::Ledger, DetailType::All, Direction::In);
        stats.inc_dir_aggregate(StatType::Ledger, DetailType::Send, Direction::In);
        stats.inc_dir_aggregate(StatType::Ledger, DetailType::Send, Direction::In);
        stats.inc_dir_aggregate(StatType::Ledger, DetailType::Receive, Direction::In);
        assert_eq!(
            10,
            stats.count(StatType::Ledger, DetailType::All, Direction::In)
        );
        assert_eq!(
            2,
            stats.count(StatType::Ledger, DetailType::Send, Direction::In)
        );
        assert_eq!(
            1,
            stats.count(StatType::Ledger, DetailType::Receive, Direction::In)
        );
    }

    #[test]
    fn samples() {
        let stats = Stats::new(StatsConfig::new());
        stats.sample(Sample::ActiveElectionDuration, 5, (1, 10));
        stats.sample(Sample::ActiveElectionDuration, 5, (1, 10));
        stats.sample(Sample::ActiveElectionDuration, 11, (1, 10));
        stats.sample(Sample::ActiveElectionDuration, 37, (1, 10));

        stats.sample(Sample::BootstrapTagDuration, 2137, (1, 10));

        let samples1 = stats.samples(Sample::ActiveElectionDuration);
        assert_eq!(samples1, [5, 5, 11, 37]);

        let samples2 = stats.samples(Sample::ActiveElectionDuration);
        assert!(samples2.is_empty());

        stats.sample(Sample::ActiveElectionDuration, 3, (1, 10));

        let samples3 = stats.samples(Sample::ActiveElectionDuration);
        assert_eq!(samples3, [3]);

        let samples4 = stats.samples(Sample::BootstrapTagDuration);
        assert_eq!(samples4, [2137]);
    }
}
