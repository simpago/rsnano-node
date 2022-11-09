use std::{
    cmp::min,
    mem::size_of,
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use blake2::{
    digest::{Update, VariableOutput},
    VarBlake2b,
};
#[cfg(test)]
use once_cell::sync::Lazy;

use crate::{
    core::{Root, WorkVersion},
    utils::get_cpu_count,
};

use super::{WorkThresholds, XorShift1024Star};

static NEVER_EXPIRES: AtomicI32 = AtomicI32::new(0);

struct WorkItem {
    version: WorkVersion,
    item: Root,
    difficulty: u64,
    callback: Option<Box<dyn Fn(Option<u64>) + Send>>,
}

#[derive(Clone)]
pub struct WorkTicket<'a> {
    ticket: &'a AtomicI32,
    ticket_copy: i32,
}

impl<'a> WorkTicket<'a> {
    pub fn never_expires() -> Self {
        Self::new(&NEVER_EXPIRES)
    }

    pub fn new(ticket: &'a AtomicI32) -> Self {
        Self {
            ticket,
            ticket_copy: ticket.load(Ordering::SeqCst),
        }
    }

    pub fn expired(&self) -> bool {
        self.ticket_copy != self.ticket.load(Ordering::SeqCst)
    }
}

struct WorkQueue(Vec<WorkItem>);

impl WorkQueue {
    pub fn new() -> Self {
        WorkQueue(Vec::new())
    }

    pub fn first(&self) -> Option<&WorkItem> {
        self.0.first()
    }

    pub fn is_first(&self, root: &Root) -> bool {
        if let Some(front) = self.first() {
            front.item == *root
        } else {
            false
        }
    }

    pub fn cancel(&mut self, root: &Root) {
        self.0.retain(|item| {
            let retain = item.item != *root;
            if !retain {
                if let Some(callback) = &item.callback {
                    (callback)(None);
                }
            }
            retain
        });
    }

    pub fn enqueue(&mut self, item: WorkItem) {
        self.0.push(item);
    }

    pub fn dequeue(&mut self) -> WorkItem {
        self.0.remove(0)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub type OpenClWorkFunc = dyn Fn(WorkVersion, Root, u64, &WorkTicket) -> Option<u64> + Send + Sync;

struct WorkPoolState {
    opencl: Option<Box<OpenClWorkFunc>>,
    work_thresholds: WorkThresholds,
    work_queue: Mutex<WorkQueue>,
    should_stop: AtomicBool,
    producer_condition: Condvar,
    ticket: AtomicI32,
    pow_rate_limiter: Duration,
}

impl WorkPoolState {
    fn new(
        opencl: Option<Box<OpenClWorkFunc>>,
        work_thresholds: WorkThresholds,
        pow_rate_limiter: Duration,
    ) -> Self {
        Self {
            opencl,
            work_thresholds,
            work_queue: Mutex::new(WorkQueue::new()),
            should_stop: AtomicBool::new(false),
            producer_condition: Condvar::new(),
            ticket: AtomicI32::new(0),
            pow_rate_limiter,
        }
    }

    pub fn create_work_ticket(&'_ self) -> WorkTicket<'_> {
        WorkTicket::new(&self.ticket)
    }

    pub fn has_opencl(&self) -> bool {
        self.opencl.is_some()
    }

    pub fn expire_work_tickets(&self) {
        self.ticket.fetch_add(1, Ordering::SeqCst);
    }

    fn worker_thread_count(&self, max_threads: u32) -> u32 {
        let mut thread_count = min(max_threads, get_cpu_count() as u32);
        if self.opencl.is_some() {
            // One thread to handle OpenCL
            thread_count += 1;
        }
        thread_count
    }

    fn enqueue(&self, work_item: WorkItem) {
        {
            let mut pending = self.work_queue.lock().unwrap();
            pending.enqueue(work_item)
        }
        self.producer_condition.notify_all();
    }

    pub fn cancel(&self, root: &Root) {
        let mut lock = self.work_queue.lock().unwrap();
        if !self.should_stop.load(Ordering::Relaxed) {
            if lock.is_first(root) {
                self.expire_work_tickets();
            }

            lock.cancel(root);
        }
    }

    pub fn stop(&self) {
        self.should_stop.store(true, Ordering::Relaxed);
        self.expire_work_tickets();
        self.producer_condition.notify_all();
    }
}

pub struct WorkPool {
    max_threads: u32,
    threads: Vec<JoinHandle<()>>,
    shared_state: Arc<WorkPoolState>,
}

impl WorkPool {
    pub fn new(
        work_thresholds: WorkThresholds,
        max_threads: u32,
        pow_rate_limiter: Duration,
        opencl: Option<Box<OpenClWorkFunc>>,
    ) -> Self {
        let shared_state = Arc::new(WorkPoolState::new(
            opencl,
            work_thresholds,
            pow_rate_limiter,
        ));

        Self {
            max_threads,
            threads: create_worker_threads(&shared_state, max_threads),
            shared_state,
        }
    }

    pub fn has_opencl(&self) -> bool {
        self.shared_state.opencl.is_some()
    }

    pub fn cancel(&self, root: &Root) {
        self.shared_state.cancel(root);
    }

    pub fn stop(&self) {
        self.shared_state.stop();
    }

    pub fn generate_async(
        &self,
        version: WorkVersion,
        root: Root,
        difficulty: u64,
        done: Option<Box<dyn Fn(Option<u64>) + Send>>,
    ) {
        debug_assert!(!root.is_zero());
        if !self.threads.is_empty() {
            self.shared_state.enqueue(WorkItem {
                version,
                item: root,
                difficulty,
                callback: done,
            });
        } else if let Some(callback) = done {
            callback(None);
        }
    }

    pub fn generate_dev(&self, root: Root, difficulty: u64) -> Option<u64> {
        self.generate(WorkVersion::Work1, root, difficulty)
    }

    pub fn generate_dev2(&self, root: Root) -> Option<u64> {
        self.generate(
            WorkVersion::Work1,
            root,
            self.shared_state.work_thresholds.base,
        )
    }

    pub fn generate(&self, version: WorkVersion, root: Root, difficulty: u64) -> Option<u64> {
        if self.threads.is_empty() {
            return None;
        }

        let done_notifier = WorkDoneNotifier::new();
        let done_notifier_clone = done_notifier.clone();

        self.generate_async(
            version,
            root,
            difficulty,
            Some(Box::new(move |work| {
                done_notifier_clone.signal_done(work);
            })),
        );

        done_notifier.wait()
    }

    pub fn size(&self) -> usize {
        self.shared_state.work_queue.lock().unwrap().len()
    }

    pub fn pending_value_size() -> usize {
        size_of::<WorkItem>()
    }

    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }

    pub fn threshold_base(&self, version: WorkVersion) -> u64 {
        self.shared_state.work_thresholds.threshold_base(version)
    }

    pub fn difficulty(&self, version: WorkVersion, root: &Root, work: u64) -> u64 {
        self.shared_state
            .work_thresholds
            .difficulty(version, root, work)
    }
}

#[derive(Default)]
struct WorkDoneState {
    work: Option<u64>,
    done: bool,
}

#[derive(Clone)]
struct WorkDoneNotifier {
    state: Arc<(Mutex<WorkDoneState>, Condvar)>,
}

impl WorkDoneNotifier {
    fn new() -> Self {
        Self {
            state: Arc::new((Mutex::new(WorkDoneState::default()), Condvar::new())),
        }
    }

    fn signal_done(&self, work: Option<u64>) {
        {
            let mut lock = self.state.0.lock().unwrap();
            lock.work = work;
            lock.done = true;
        }
        self.state.1.notify_one();
    }

    fn wait(&self) -> Option<u64> {
        let mut lock = self.state.0.lock().unwrap();
        loop {
            if lock.done {
                return lock.work;
            }
            lock = self.state.1.wait(lock).unwrap();
        }
    }
}

fn create_worker_threads(
    shared_state: &Arc<WorkPoolState>,
    max_threads: u32,
) -> Vec<JoinHandle<()>> {
    let thread_count = shared_state.worker_thread_count(max_threads);
    (0..thread_count)
        .map(|i| create_worker_thread(i, shared_state))
        .collect()
}

fn create_worker_thread(thread_number: u32, shared_state: &Arc<WorkPoolState>) -> JoinHandle<()> {
    let state = Arc::clone(&shared_state);
    thread::Builder::new()
        .name("Work pool".to_string())
        .spawn(move || {
            WorkThread::new(thread_number, state).work_loop();
        })
        .unwrap()
}

impl Drop for WorkPool {
    fn drop(&mut self) {
        self.stop();
        for handle in self.threads.drain(..) {
            handle.join().unwrap();
        }
    }
}

struct WorkThread {
    thread_number: u32,
    state: Arc<WorkPoolState>,

    // Quick RNG for work attempts.
    rng: XorShift1024Star,
    work: u64,
    output: u64,
    hasher: VarBlake2b,
}

impl WorkThread {
    fn new(thread_number: u32, state: Arc<WorkPoolState>) -> Self {
        Self {
            thread_number,
            state,
            rng: XorShift1024Star::new(),
            work: 0,
            output: 0,
            hasher: VarBlake2b::new_keyed(&[], size_of::<u64>()),
        }
    }
}

impl WorkThread {
    fn work_loop(mut self) {
        let mut pending = self.state.work_queue.lock().unwrap();
        while !self.state.should_stop.load(Ordering::Relaxed) {
            if let Some(current) = pending.first() {
                let current_version = current.version;
                let current_item = current.item;
                let current_difficulty = current.difficulty;
                let ticket_l = self.state.create_work_ticket();
                drop(pending);
                self.output = 0;
                let mut opt_work = None;
                if self.thread_number == 0 && self.state.has_opencl() {
                    opt_work = (self.state.opencl.as_ref().unwrap())(
                        current_version,
                        current_item,
                        current_difficulty,
                        &ticket_l,
                    );
                }
                if let Some(w) = opt_work {
                    self.work = w;
                    self.output = self.state.work_thresholds.value(&current_item, self.work);
                } else {
                    while !ticket_l.expired() && self.output < current_difficulty {
                        // Don't query main memory every iteration in order to reduce memory bus traffic
                        // All operations here operate on stack memory
                        // Count iterations down to zero since comparing to zero is easier than comparing to another number
                        let mut iteration = 256u32;
                        while iteration > 0 && self.output < current_difficulty {
                            self.work = self.rng.next();
                            self.hasher.update(&self.work.to_le_bytes());
                            self.hasher.update(current_item.as_bytes());
                            self.hasher.finalize_variable_reset(|result| {
                                self.output = u64::from_le_bytes(result.try_into().unwrap());
                            });
                            iteration -= 1;
                        }

                        // Add a rate limiter (if specified) to the pow calculation to save some CPUs which don't want to operate at full throttle
                        if !self.state.pow_rate_limiter.is_zero() {
                            thread::sleep(self.state.pow_rate_limiter);
                        }
                    }
                }
                pending = self.state.work_queue.lock().unwrap();
                if !ticket_l.expired() {
                    // If the ticket matches what we started with, we're the ones that found the solution
                    debug_assert!(self.output >= current_difficulty);
                    debug_assert!(
                        current_difficulty == 0
                            || self.state.work_thresholds.value(&current_item, self.work)
                                == self.output
                    );
                    // Signal other threads to stop their work next time they check ticket
                    self.state.expire_work_tickets();
                    let current_l = pending.dequeue();
                    drop(pending);
                    if let Some(callback) = current_l.callback {
                        (callback)(Some(self.work));
                    }
                    pending = self.state.work_queue.lock().unwrap();
                } else {
                    // A different thread found a solution
                }
            } else {
                // Wait for a work request
                pending = self.state.producer_condition.wait(pending).unwrap();
            }
        }
    }
}

#[cfg(test)]
pub(crate) static DEV_WORK_POOL: Lazy<WorkPool> = Lazy::new(|| {
    WorkPool::new(
        crate::DEV_NETWORK_PARAMS.work.clone(),
        u32::MAX,
        Duration::ZERO,
        None,
    )
});

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use crate::{
        core::{Block, BlockBuilder},
        DEV_NETWORK_PARAMS,
    };

    use super::*;

    #[test]
    fn work_disabled() {
        let pool = WorkPool::new(
            DEV_NETWORK_PARAMS.network.work.clone(),
            0,
            Duration::ZERO,
            None,
        );
        let result = pool.generate_dev2(Root::from(1));
        assert_eq!(result, None);
    }

    #[test]
    fn work_one() {
        let pool = &DEV_WORK_POOL;
        let mut block = BlockBuilder::state().build().unwrap();
        block.set_work(pool.generate_dev2(block.root()).unwrap());
        assert!(pool.threshold_base(block.work_version()) < difficulty(&block));
    }

    #[test]
    fn work_validate() {
        let pool = &DEV_WORK_POOL;
        let mut block = BlockBuilder::send().work(6).build().unwrap();
        assert!(difficulty(&block) < pool.threshold_base(block.work_version()));
        block.set_work(pool.generate_dev2(block.root()).unwrap());
        assert!(difficulty(&block) > pool.threshold_base(block.work_version()));
    }

    #[test]
    fn work_cancel() {
        let (tx, rx) = mpsc::channel();
        let key = Root::from(12345);
        DEV_WORK_POOL.generate_async(
            WorkVersion::Work1,
            key,
            DEV_NETWORK_PARAMS.network.work.base,
            Some(Box::new(move |_done| {
                tx.send(()).unwrap();
            })),
        );
        DEV_WORK_POOL.cancel(&key);
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)), Ok(()))
    }

    #[test]
    fn work_difficulty() {
        let root = Root::from(1);
        let difficulty1 = 0xff00000000000000;
        let difficulty2 = 0xfff0000000000000;
        let difficulty3 = 0xffff000000000000;
        let mut result_difficulty = u64::MAX;

        while result_difficulty > difficulty2 {
            let work = DEV_WORK_POOL
                .generate(WorkVersion::Work1, root, difficulty1)
                .unwrap();
            result_difficulty = DEV_NETWORK_PARAMS
                .work
                .difficulty(WorkVersion::Work1, &root, work);
        }
        assert!(result_difficulty > difficulty1);

        result_difficulty = u64::MAX;
        while result_difficulty > difficulty3 {
            let work = DEV_WORK_POOL
                .generate(WorkVersion::Work1, root, difficulty2)
                .unwrap();
            result_difficulty = DEV_NETWORK_PARAMS
                .work
                .difficulty(WorkVersion::Work1, &root, work);
        }
        assert!(result_difficulty > difficulty2);
    }

    fn difficulty(block: &dyn Block) -> u64 {
        DEV_NETWORK_PARAMS.network.work.difficulty_block(block)
    }
}
