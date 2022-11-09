use std::sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    Condvar, Mutex, MutexGuard,
};

use crate::{
    core::{Root, WorkVersion},
    work::WorkThresholds,
};

use super::WorkTicket;

pub(crate) struct WorkItem {
    pub version: WorkVersion,
    pub item: Root,
    pub min_difficulty: u64,
    pub callback: Option<Box<dyn Fn(Option<u64>) + Send>>,
}

impl WorkItem {
    pub fn work_found(&self, work: u64, difficulty: u64) {
        // we're the ones that found the solution
        debug_assert!(difficulty >= self.min_difficulty);
        debug_assert!(
            self.min_difficulty == 0
                || WorkThresholds::difficulty_v1(&self.item, work) == difficulty
        );
        if let Some(callback) = &self.callback {
            (callback)(Some(work));
        }
    }
}

pub(crate) struct WorkQueue(Vec<WorkItem>);

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

/// Coordinates access to the work queue between multiple threads
pub(crate) struct WorkQueueCoordinator {
    work_queue: Mutex<WorkQueue>,
    should_stop: AtomicBool,
    producer_condition: Condvar,
    ticket: AtomicI32,
}

impl WorkQueueCoordinator {
    pub fn new() -> Self {
        Self {
            work_queue: Mutex::new(WorkQueue::new()),
            should_stop: AtomicBool::new(false),
            producer_condition: Condvar::new(),
            ticket: AtomicI32::new(0),
        }
    }

    pub fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::Relaxed)
    }

    pub fn lock_work_queue(&self) -> MutexGuard<WorkQueue> {
        self.work_queue.lock().unwrap()
    }

    pub fn wait_for_new_work_item<'a>(
        &'a self,
        guard: MutexGuard<'a, WorkQueue>,
    ) -> MutexGuard<'a, WorkQueue> {
        self.producer_condition.wait(guard).unwrap()
    }

    pub fn enqueue(&self, work_item: WorkItem) {
        {
            let mut pending = self.work_queue.lock().unwrap();
            pending.enqueue(work_item)
        }
        self.producer_condition.notify_all();
    }

    pub fn notify_new_work_ticket(&self) {
        self.producer_condition.notify_all()
    }

    pub fn stop(&self) {
        self.should_stop.store(true, Ordering::Relaxed);
        self.expire_work_tickets();
        self.notify_new_work_ticket();
    }

    pub fn create_work_ticket(&'_ self) -> WorkTicket<'_> {
        WorkTicket::new(&self.ticket)
    }

    pub fn expire_work_tickets(&self) {
        self.ticket.fetch_add(1, Ordering::SeqCst);
    }

    pub fn cancel(&self, root: &Root) {
        let mut lock = self.lock_work_queue();
        if !self.should_stop() {
            if lock.is_first(root) {
                self.expire_work_tickets();
            }

            lock.cancel(root);
        }
    }
}
