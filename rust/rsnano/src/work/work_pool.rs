use std::{
    mem::size_of,
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[cfg(test)]
use once_cell::sync::Lazy;

use super::{
    work_queue::WorkQueueCoordinator, CpuWorkGenerator, OpenClWorkFunc, OpenClWorkGenerator,
    WorkItem, WorkThread, WorkThresholds,
};
use crate::core::{Root, WorkVersion};

static NEVER_EXPIRES: AtomicI32 = AtomicI32::new(0);

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

pub struct WorkPool {
    threads: Vec<JoinHandle<()>>,
    work_queue: Arc<WorkQueueCoordinator>,
    work_thresholds: WorkThresholds,
    pow_rate_limiter: Duration,
    has_opencl: bool,
}

impl WorkPool {
    pub fn new(
        work_thresholds: WorkThresholds,
        thread_count: usize,
        pow_rate_limiter: Duration,
        opencl: Option<Box<OpenClWorkFunc>>,
    ) -> Self {
        let mut pool = Self {
            threads: Vec::new(),
            work_queue: Arc::new(WorkQueueCoordinator::new()),
            work_thresholds,
            has_opencl: false,
            pow_rate_limiter,
        };

        pool.spawn_threads(thread_count, opencl);
        pool
    }

    fn spawn_threads(&mut self, thread_count: usize, opencl: Option<Box<OpenClWorkFunc>>) {
        if let Some(opencl) = opencl {
            // One extra thread to handle OpenCL
            self.threads.push(self.spawn_open_cl_thread(opencl))
        }

        for _ in 0..thread_count {
            self.threads.push(self.spawn_cpu_worker_thread())
        }
    }

    fn spawn_open_cl_thread(&self, opencl: Box<OpenClWorkFunc>) -> JoinHandle<()> {
        self.spawn_worker_thread(OpenClWorkGenerator::new(self.pow_rate_limiter, opencl))
    }

    fn spawn_cpu_worker_thread(&self) -> JoinHandle<()> {
        self.spawn_worker_thread(CpuWorkGenerator::new(self.pow_rate_limiter))
    }

    fn spawn_worker_thread<T>(&self, work_generator: T) -> JoinHandle<()>
    where
        T: WorkGenerator + Send + Sync + 'static,
    {
        let work_queue = Arc::clone(&self.work_queue);
        thread::Builder::new()
            .name("Work pool".to_string())
            .spawn(move || {
                WorkThread::new(work_generator, work_queue).work_loop();
            })
            .unwrap()
    }

    pub fn has_opencl(&self) -> bool {
        self.has_opencl
    }

    pub fn cancel(&self, root: &Root) {
        self.work_queue.cancel(root);
    }

    pub fn stop(&self) {
        self.work_queue.stop();
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
            self.work_queue.enqueue(WorkItem {
                version,
                item: root,
                min_difficulty: difficulty,
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
        self.generate(WorkVersion::Work1, root, self.work_thresholds.base)
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
        self.work_queue.lock_work_queue().len()
    }

    pub fn pending_value_size() -> usize {
        size_of::<WorkItem>()
    }

    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }

    pub fn threshold_base(&self, version: WorkVersion) -> u64 {
        self.work_thresholds.threshold_base(version)
    }

    pub fn difficulty(&self, version: WorkVersion, root: &Root, work: u64) -> u64 {
        self.work_thresholds.difficulty(version, root, work)
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

impl Drop for WorkPool {
    fn drop(&mut self) {
        self.stop();
        for handle in self.threads.drain(..) {
            handle.join().unwrap();
        }
    }
}

pub(crate) trait WorkGenerator {
    fn create(
        &mut self,
        version: WorkVersion,
        item: &Root,
        min_difficulty: u64,
        work_ticket: &WorkTicket,
    ) -> Option<(u64, u64)>;
}

#[cfg(test)]
pub(crate) static DEV_WORK_POOL: Lazy<WorkPool> = Lazy::new(|| {
    WorkPool::new(
        crate::DEV_NETWORK_PARAMS.work.clone(),
        crate::utils::get_cpu_count(),
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
