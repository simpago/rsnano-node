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
    generic_queue: Mutex<VecDeque<Entry>>,
    bootstrap_queue: Mutex<VecDeque<Entry>>,
    notify_enqueued: Notify,
    notify_dequeued: Notify,
    closed: Arc<AtomicBool>,
}

impl WriteQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            generic_queue: Mutex::new(VecDeque::with_capacity(max_size * 2)),
            bootstrap_queue: Mutex::new(VecDeque::with_capacity(max_size * 2)),
            notify_enqueued: Notify::new(),
            notify_dequeued: Notify::new(),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn insert(&self, buffer: Arc<Vec<u8>>, traffic_type: TrafficType) {
        let queue = self.queue_for(traffic_type);

        loop {
            if self.closed.load(Ordering::SeqCst) {
                return;
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
    }

    /// returns: inserted
    pub fn try_insert(&self, buffer: Arc<Vec<u8>>, traffic_type: TrafficType) -> bool {
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

        inserted
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

    pub async fn pop(&self) -> Option<(Entry, TrafficType)> {
        let mut entry;
        let traffic_type;

        loop {
            if self.closed.load(Ordering::SeqCst) {
                return None;
            }

            // always prefer generic queue!
            {
                let mut guard = self.generic_queue.lock().unwrap();
                entry = guard.pop_front();
                if entry.is_some() {
                    traffic_type = TrafficType::Generic;
                    break;
                }
            }

            {
                let mut guard = self.bootstrap_queue.lock().unwrap();
                entry = guard.pop_front();
                if entry.is_some() {
                    traffic_type = TrafficType::Bootstrap;
                    break;
                }
            }

            self.notify_enqueued.notified().await;
        }

        let entry = entry?;
        self.notify_dequeued.notify_one();
        Some((entry, traffic_type))
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

pub struct Entry {
    pub buffer: Arc<Vec<u8>>,
}
