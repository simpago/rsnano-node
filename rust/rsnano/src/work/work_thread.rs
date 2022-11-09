use super::{WorkGenerator, WorkQueueCoordinator};
use std::sync::Arc;

pub(crate) struct WorkThread<T>
where
    T: WorkGenerator + Send + Sync,
{
    work_queue: Arc<WorkQueueCoordinator>,
    work_generator: T,
}

/// A single thread to generate PoW
impl<T> WorkThread<T>
where
    T: WorkGenerator + Send + Sync,
{
    pub fn new(work_generator: T, work_queue: Arc<WorkQueueCoordinator>) -> Self {
        Self {
            work_generator,
            work_queue,
        }
    }

    pub fn work_loop(mut self) {
        let mut work_queue = self.work_queue.lock_work_queue();
        while !self.work_queue.should_stop() {
            if let Some(current) = work_queue.first() {
                let version = current.version;
                let item = current.item;
                let min_difficulty = current.min_difficulty;
                let work_ticket = self.work_queue.create_work_ticket();

                // drop work_queue lock, because work generation will take some time
                drop(work_queue);

                let result =
                    self.work_generator
                        .create(version, &item, min_difficulty, &work_ticket);

                work_queue = self.work_queue.lock_work_queue();

                if let Some((work, difficulty)) = result {
                    if !work_ticket.expired() {
                        // Signal other threads to stop their work next time they check their ticket
                        self.work_queue.expire_work_tickets();
                        let current = work_queue.dequeue();

                        // work_found callback can take some time, to let's drop the lock
                        drop(work_queue);
                        current.work_found(work, difficulty);
                        work_queue = self.work_queue.lock_work_queue();
                    }
                } else {
                    // A different thread found a solution
                }
            } else {
                work_queue = self.work_queue.wait_for_new_work_item(work_queue);
            }
        }
    }
}
