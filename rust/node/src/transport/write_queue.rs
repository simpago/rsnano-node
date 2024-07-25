use super::TrafficType;
use crate::utils::ErrorCode;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub type WriteCallback = Box<dyn FnOnce(ErrorCode, usize) + Send>;

pub(crate) struct Entry {
    pub buffer: Arc<Vec<u8>>,
    pub callback: Option<WriteCallback>,
}

struct Queues {
    generic_queue: VecDeque<Entry>,
    bootstrap_queue: VecDeque<Entry>,
}

impl Queues {
    fn get(&self, traffic_type: TrafficType) -> &VecDeque<Entry> {
        match traffic_type {
            TrafficType::Generic => &self.generic_queue,
            TrafficType::Bootstrap => &self.bootstrap_queue,
        }
    }

    fn get_mut(&mut self, traffic_type: TrafficType) -> &mut VecDeque<Entry> {
        match traffic_type {
            TrafficType::Generic => &mut self.generic_queue,
            TrafficType::Bootstrap => &mut self.bootstrap_queue,
        }
    }
}

pub(crate) struct WriteQueue {
    max_size: usize,
    queues: Mutex<Queues>,
}

impl WriteQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            queues: Mutex::new(Queues {
                generic_queue: VecDeque::new(),
                bootstrap_queue: VecDeque::new(),
            }),
        }
    }

    pub fn insert(
        &self,
        buffer: Arc<Vec<u8>>,
        callback: Option<WriteCallback>,
        traffic_type: TrafficType,
    ) -> (bool, Option<WriteCallback>) {
        let mut queues = self.queues.lock().unwrap();
        let queue = queues.get_mut(traffic_type);
        if queue.len() < 2 * self.max_size {
            queue.push_back(Entry { buffer, callback });
            (true, None) // Queued
        } else {
            (false, callback) // Not queued
        }
    }

    pub fn pop(&self) -> Option<Entry> {
        let mut queues = self.queues.lock().unwrap();

        // TODO: This is a very basic prioritization, implement something more advanced and configurable
        let item = queues.generic_queue.pop_front();
        if item.is_some() {
            item
        } else {
            queues.bootstrap_queue.pop_front()
        }
    }

    pub fn clear(&self) {
        let mut queues = self.queues.lock().unwrap();
        queues.generic_queue.clear();
        queues.bootstrap_queue.clear();
    }

    pub fn size(&self, traffic_type: TrafficType) -> usize {
        let queues = self.queues.lock().unwrap();
        queues.get(traffic_type).len()
    }
}
