use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
};

/** Distinct areas write locking is done, order is irrelevant */
#[derive(FromPrimitive, Clone, Copy, PartialEq, Eq)]
pub enum Writer {
    ConfirmationHeight,
    ProcessBatch,
    Pruning,
    Testing, // Used in tests to emulate a write lock
}

pub struct WriteGuard {
    guard_finish_callback: Option<Arc<dyn Fn()>>,
}

impl WriteGuard {
    pub fn new(guard_finish_callback: Arc<dyn Fn()>) -> Self {
        Self {
            guard_finish_callback: Some(guard_finish_callback),
        }
    }

    pub fn release(&mut self) {
        if let Some(callback) = self.guard_finish_callback.take() {
            callback();
        }
    }

    pub fn null() -> Self {
        Self {
            guard_finish_callback: None,
        }
    }
}

impl Drop for WriteGuard {
    fn drop(&mut self) {
        self.release();
    }
}

pub struct WriteDatabaseQueue {
    data: Arc<WriteDatabaseQueueData>,
    guard_finish_callback: Arc<dyn Fn()>,
}

struct WriteDatabaseQueueData {
    queue: Mutex<VecDeque<Writer>>,
    use_noops: bool,
    condition: Condvar,
}

impl WriteDatabaseQueue {
    pub fn new(use_noops: bool) -> Self {
        let data = Arc::new(WriteDatabaseQueueData {
            queue: Mutex::new(VecDeque::new()),
            use_noops,
            condition: Condvar::new(),
        });

        let data_clone = data.clone();

        Self {
            data,
            guard_finish_callback: Arc::new(move || {
                if !data_clone.use_noops {
                    let mut guard = data_clone.queue.lock().unwrap();
                    guard.pop_front();
                }
                data_clone.condition.notify_all();
            }),
        }
    }

    /// Blocks until we are at the head of the queue
    pub fn wait(&self, writer: Writer) -> WriteGuard {
        if self.data.use_noops {
            return WriteGuard::null();
        }

        let mut lk = self.data.queue.lock().unwrap();
        // Add writer to the end of the queue if it's not already waiting
        if !lk.contains(&writer) {
            lk.push_back(writer);
        }

        while let Some(&w) = lk.front() {
            if w != writer {
                lk = self.data.condition.wait(lk).unwrap();
            } else {
                break;
            }
        }

        self.create_write_guard()
    }

    /// Returns true if this writer is now at the front of the queue
    pub fn process(&self, writer: Writer) -> bool {
        if self.data.use_noops {
            return true;
        }

        let result = {
            let mut guard = self.data.queue.lock().unwrap();
            // Add writer to the end of the queue if it's not already waiting
            if !guard.contains(&writer) {
                guard.push_back(writer);
            }

            *guard.front().unwrap() == writer
        };

        result
    }

    fn create_write_guard(&self) -> WriteGuard {
        WriteGuard::new(Arc::clone(&self.guard_finish_callback))
    }

    /// Returns true if this writer is anywhere in the queue. Currently only used in tests
    pub fn contains(&self, writer: Writer) -> bool {
        debug_assert!(!self.data.use_noops);
        self.data.queue.lock().unwrap().contains(&writer)
    }

    /// Doesn't actually pop anything until the returned write_guard is out of scope
    pub fn pop(&self) -> WriteGuard {
        self.create_write_guard()
    }
}
