use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
};

/** Distinct areas write locking is done, order is irrelevant */
#[derive(FromPrimitive, Clone, Copy, PartialEq, Eq)]
pub enum Writer {
    ConfirmationHeight,
    BlockProcessor,
    Pruning,
    VotingFinal,
    Testing, // Used in tests to emulate a write lock
}

pub struct WriteGuard {
    pub writer: Writer,
    guard_finish_callback: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl WriteGuard {
    pub fn new(writer: Writer, guard_finish_callback: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            writer,
            guard_finish_callback: Some(guard_finish_callback),
        }
    }

    pub fn release(&mut self) {
        if let Some(callback) = self.guard_finish_callback.take() {
            callback();
        }
    }

    pub fn is_owned(&self) -> bool {
        self.guard_finish_callback.is_some()
    }

    pub fn null() -> Self {
        Self {
            writer: Writer::Testing,
            guard_finish_callback: None,
        }
    }
}

impl Drop for WriteGuard {
    fn drop(&mut self) {
        self.release();
    }
}

pub struct WriteQueue {
    data: Arc<WriteQueueData>,
    guard_finish_callback: Arc<dyn Fn() + Send + Sync>,
}

struct WriteQueueData {
    queue: Mutex<VecDeque<Writer>>,
    condition: Condvar,
}

impl WriteQueue {
    pub fn new() -> Self {
        let data = Arc::new(WriteQueueData {
            queue: Mutex::new(VecDeque::new()),
            condition: Condvar::new(),
        });

        let data_clone = data.clone();

        Self {
            data,
            guard_finish_callback: Arc::new(move || {
                let mut guard = data_clone.queue.lock().unwrap();
                guard.pop_front();
                data_clone.condition.notify_all();
            }),
        }
    }

    /// Blocks until we are at the head of the queue and blocks other waiters until write_guard goes out of scope
    pub fn wait(&self, writer: Writer) -> WriteGuard {
        let mut lk = self.data.queue.lock().unwrap();
        assert!(lk.iter().all(|i| *i != writer));
        lk.push_back(writer);

        let _result = self
            .data
            .condition
            .wait_while(lk, |queue| queue.front() != Some(&writer));

        self.create_write_guard(writer)
    }

    /// Returns true if this writer is anywhere in the queue. Currently only used in tests
    pub fn contains(&self, writer: Writer) -> bool {
        self.data.queue.lock().unwrap().contains(&writer)
    }

    fn create_write_guard(&self, writer: Writer) -> WriteGuard {
        WriteGuard::new(writer, Arc::clone(&self.guard_finish_callback))
    }
}
