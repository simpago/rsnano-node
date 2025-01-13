use crate::TrafficType;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::Notify;

pub struct WriteQueue {
    generic_queue: Arc<Mutex<VecDeque<Entry>>>,
    bootstrap_queue: Arc<Mutex<VecDeque<Entry>>>,
    notify_enqueued: Arc<Notify>,
    notify_dequeued: Arc<Notify>,
    closed: Arc<AtomicBool>,
}

impl WriteQueue {
    pub fn new(max_size: usize) -> (Self, WriteQueueReceiver) {
        let notify_enqueued = Arc::new(Notify::new());
        let notify_dequeued = Arc::new(Notify::new());
        let closed = Arc::new(AtomicBool::new(false));
        let receiver = WriteQueueReceiver {
            generic: Arc::new(Mutex::new(VecDeque::with_capacity(max_size * 2))),
            bootstrap: Arc::new(Mutex::new(VecDeque::with_capacity(max_size * 2))),
            enqueued: notify_enqueued.clone(),
            dequeued: notify_dequeued.clone(),
            closed: closed.clone(),
        };
        (
            Self {
                generic_queue: receiver.generic.clone(),
                bootstrap_queue: receiver.bootstrap.clone(),
                notify_enqueued,
                notify_dequeued,
                closed,
            },
            receiver,
        )
    }

    pub async fn insert(
        &self,
        buffer: Arc<Vec<u8>>,
        traffic_type: TrafficType,
    ) -> anyhow::Result<()> {
        let queue = self.queue_for(traffic_type);

        loop {
            if self.closed.load(Ordering::SeqCst) {
                return Ok(());
            }

            {
                let mut guard = queue.lock().unwrap();
                if guard.capacity() > 0 {
                    let entry = Entry { buffer };
                    guard.push_back(entry);
                    break;
                }
            }

            self.notify_dequeued.notified().await;
        }

        self.notify_enqueued.notify_one();
        // TODO return ()
        Ok(())
    }

    /// returns: inserted | write_error
    pub fn try_insert(&self, buffer: Arc<Vec<u8>>, traffic_type: TrafficType) -> (bool, bool) {
        let queue = self.queue_for(traffic_type);
        let inserted;
        {
            let mut guard = queue.lock().unwrap();
            if guard.capacity() > 0 {
                let entry = Entry { buffer };
                guard.push_back(entry);
                inserted = true;
            } else {
                inserted = false;
            }
        }

        if inserted {
            self.notify_enqueued.notify_one();
        }

        // TODO remove unused write error return
        (inserted, false)
    }

    pub fn capacity(&self, traffic_type: TrafficType) -> usize {
        self.queue_for(traffic_type).lock().unwrap().capacity()
    }

    fn queue_for(&self, traffic_type: TrafficType) -> &Mutex<VecDeque<Entry>> {
        match traffic_type {
            TrafficType::Generic => &self.generic_queue,
            TrafficType::Bootstrap => &self.bootstrap_queue,
        }
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.notify_enqueued.notify_one();
        self.notify_dequeued.notify_one();
    }
}

impl Drop for WriteQueue {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct WriteQueueReceiver {
    generic: Arc<Mutex<VecDeque<Entry>>>,
    bootstrap: Arc<Mutex<VecDeque<Entry>>>,
    enqueued: Arc<Notify>,
    dequeued: Arc<Notify>,
    closed: Arc<AtomicBool>,
}

impl WriteQueueReceiver {
    pub async fn pop(&mut self) -> Option<(Entry, TrafficType)> {
        let mut entry;
        let traffic_type;

        loop {
            if self.closed.load(Ordering::SeqCst) {
                return None;
            }

            // always prefer generic queue!
            {
                let mut guard = self.generic.lock().unwrap();
                entry = guard.pop_front();
                if entry.is_some() {
                    traffic_type = TrafficType::Generic;
                    break;
                }
            }

            {
                let mut guard = self.bootstrap.lock().unwrap();
                entry = guard.pop_front();
                if entry.is_some() {
                    traffic_type = TrafficType::Bootstrap;
                    break;
                }
            }

            self.enqueued.notified().await;
        }

        let entry = entry?;
        self.dequeued.notify_one();
        Some((entry, traffic_type))
    }
}

pub struct Entry {
    pub buffer: Arc<Vec<u8>>,
}
