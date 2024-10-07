use std::{
    collections::VecDeque,
    ops::DerefMut,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex, MutexGuard,
    },
    thread::{self, JoinHandle},
};

use crate::stats::{DetailType, Direction, StatType, Stats};

/**
 * Queue that processes enqueued elements in (possibly parallel) batches
 */
pub struct ProcessingQueue<T: Send + 'static> {
    thread_name: String,
    thread_count: usize,
    max_queue_size: usize,
    threads: Mutex<Vec<JoinHandle<()>>>,
    shared_state: Arc<SharedState<T>>,
    stats: Arc<Stats>,
    stat_type: StatType,
}

impl<T: Send + 'static> ProcessingQueue<T> {
    /**
     * @param thread_count Number of processing threads
     * @param max_queue_size Max number of items enqueued, items beyond this value will be discarded
     * @param max_batch_size Max number of elements processed in single batch, 0 for unlimited (default)
     */
    pub fn new(
        stats: Arc<Stats>,
        stat_type: StatType,
        thread_name: String,
        thread_count: usize,
        max_queue_size: usize,
        max_batch_size: usize,
        process_batch: Box<dyn Fn(VecDeque<T>) + Send + Sync>,
    ) -> Self {
        Self {
            thread_name,
            thread_count,
            stats: Arc::clone(&stats),
            stat_type,
            max_queue_size,
            threads: Mutex::new(Vec::with_capacity(thread_count)),
            shared_state: Arc::new(SharedState::new(
                max_batch_size,
                stats,
                stat_type,
                process_batch,
            )),
        }
    }

    pub fn start(&self) {
        let mut threads = self.threads.lock().unwrap();
        for _ in 0..self.thread_count {
            let state = Arc::clone(&self.shared_state);
            threads.push(
                thread::Builder::new()
                    .name(self.thread_name.clone())
                    .spawn(move || state.run())
                    .unwrap(),
            )
        }
    }

    pub fn stop(&self) {
        {
            let _guard = self.shared_state.queue.lock().unwrap();
            self.shared_state.stopped.store(true, Ordering::SeqCst);
        }
        self.shared_state.condition.notify_all();
        let threads = {
            let mut t = Vec::new();
            let mut guard = self.threads.lock().unwrap();
            std::mem::swap(guard.deref_mut(), &mut t);
            t
        };
        for thread in threads {
            thread.join().unwrap();
        }
    }

    /// Queues item for batch processing
    pub fn add(&self, item: T) {
        let mut queue = self.shared_state.queue.lock().unwrap();
        if queue.len() < self.max_queue_size {
            queue.push_back(item);
            drop(queue);
            self.shared_state.condition.notify_one();
            self.stats
                .inc_dir(self.stat_type, DetailType::Queue, Direction::In);
        } else {
            self.stats
                .inc_dir(self.stat_type, DetailType::Overfill, Direction::In);
        }
    }

    pub fn len(&self) -> usize {
        self.shared_state.queue.lock().unwrap().len()
    }
}

impl<T: Send + 'static> Drop for ProcessingQueue<T> {
    fn drop(&mut self) {
        self.stop()
    }
}

struct SharedState<T> {
    condition: Condvar,
    queue: Mutex<VecDeque<T>>,
    stopped: AtomicBool,
    max_batch_size: usize,
    stats: Arc<Stats>,
    stat_type: StatType,
    process_batch: Box<dyn Fn(VecDeque<T>) + Send + Sync>,
}

impl<T> SharedState<T> {
    pub fn new(
        max_batch_size: usize,
        stats: Arc<Stats>,
        stat_type: StatType,
        process_batch: Box<dyn Fn(VecDeque<T>) + Send + Sync>,
    ) -> Self {
        Self {
            condition: Condvar::new(),
            queue: Mutex::new(VecDeque::new()),
            stopped: AtomicBool::new(false),
            max_batch_size,
            stats,
            stat_type,
            process_batch,
        }
    }

    fn run(&self) {
        let mut guard = self.queue.lock().unwrap();
        while !self.stopped.load(Ordering::SeqCst) {
            let batch = self.next_batch(guard);
            if !batch.is_empty() {
                self.stats
                    .inc_dir(self.stat_type, DetailType::Batch, Direction::In);
                (self.process_batch)(batch);
            }
            guard = self.queue.lock().unwrap();
        }
    }

    fn next_batch<'a>(&self, guard: MutexGuard<'a, VecDeque<T>>) -> VecDeque<T> {
        let mut guard = self
            .condition
            .wait_while(guard, |queue| {
                queue.is_empty() && !self.stopped.load(Ordering::SeqCst)
            })
            .unwrap();

        if self.stopped.load(Ordering::SeqCst) {
            VecDeque::new()
        }
        // Unlimited batch size or queue smaller than max batch size, return the whole current queue
        else if self.max_batch_size == 0 || guard.len() < self.max_batch_size {
            let mut queue_l = VecDeque::new();
            std::mem::swap(&mut queue_l, &mut guard);
            queue_l
        }
        // Larger than max batch size, return limited number of elements
        else {
            let mut queue_l = VecDeque::with_capacity(self.max_batch_size);
            for _ in 0..self.max_batch_size {
                if let Some(item) = guard.pop_front() {
                    queue_l.push_back(item);
                }
            }

            queue_l
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn empty_queue() {
        let fixture = create_fixture();
        assert_eq!(fixture.queue.len(), 0);
    }

    #[test]
    fn process_one() {
        let fixture = create_fixture();
        fixture.queue.add(1);
        fixture.wait_until_process_count_is(1);
    }

    #[test]
    fn process_many() {
        let fixture = create_fixture();
        for _ in 0..10 {
            fixture.queue.add(1);
        }
        fixture.wait_until_process_count_is(10);
    }

    #[test]
    fn max_queue_size() {
        let fixture = create_fixture();
        fixture.queue.stop();
        for _ in 0..2 * MAX_TEST_QUEUE_LEN {
            fixture.queue.add(1);
        }
        assert_eq!(fixture.queue.len(), MAX_TEST_QUEUE_LEN)
    }

    struct TestFixture {
        processed: Arc<Mutex<usize>>,
        condition: Arc<Condvar>,
        queue: ProcessingQueue<i32>,
    }

    impl TestFixture {
        fn wait_until_process_count_is(&self, expected: usize) {
            let guard = self.processed.lock().unwrap();
            if *guard != expected {
                let (guard, result) = self
                    .condition
                    .wait_timeout_while(guard, Duration::from_secs(5), |count| *count == expected)
                    .unwrap();
                assert_eq!(result.timed_out(), false, "timeout! count was {}", *guard);
            }
        }
    }

    const MAX_TEST_QUEUE_LEN: usize = 16;

    fn create_fixture() -> TestFixture {
        let processed = Arc::new(Mutex::new(0));
        let condition = Arc::new(Condvar::new());
        let processed_clone = Arc::clone(&processed);
        let condition_clone = Arc::clone(&condition);

        let queue = ProcessingQueue::new(
            Arc::new(Stats::new(Default::default())),
            StatType::BootstrapServer,
            "processing test thread".to_string(),
            4,
            MAX_TEST_QUEUE_LEN,
            2,
            Box::new(move |i| {
                {
                    *processed_clone.lock().unwrap() += i.len();
                }
                condition_clone.notify_all();
            }),
        );
        queue.start();
        TestFixture {
            queue,
            processed,
            condition,
        }
    }
}
