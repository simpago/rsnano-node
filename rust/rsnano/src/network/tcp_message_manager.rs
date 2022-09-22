use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::{Arc, Condvar, Mutex},
};

use crate::{messages::Message, Account};

use super::SocketImpl;

pub struct TcpMessageItem {
    pub message: Option<Box<dyn Message>>,
    pub endpoint: SocketAddr,
    pub node_id: Account,
    pub socket: Option<Arc<SocketImpl>>,
}

impl Clone for TcpMessageItem {
    fn clone(&self) -> Self {
        Self {
            message: self.message.as_ref().map(|m| m.clone_box()),
            endpoint: self.endpoint,
            node_id: self.node_id,
            socket: self.socket.clone(),
        }
    }
}

impl TcpMessageItem {
    pub fn new() -> Self {
        Self {
            message: None,
            endpoint: SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
            node_id: Account::new(),
            socket: None,
        }
    }
}

impl Default for TcpMessageItem {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TcpMessageManager {
    max_entries: usize,
    state: Mutex<TcpMessageManagerState>,
    producer_condition: Condvar,
    consumer_condition: Condvar,
    blocked: Option<Box<dyn Fn() + Send + Sync>>,
}

struct TcpMessageManagerState {
    entries: VecDeque<TcpMessageItem>,
    stopped: bool,
}

impl TcpMessageManager {
    pub fn new(incoming_connections_max: usize) -> Self {
        Self {
            max_entries: incoming_connections_max * MAX_ENTRIES_PER_CONNECTION + 1,
            state: Mutex::new(TcpMessageManagerState {
                entries: VecDeque::new(),
                stopped: false,
            }),
            producer_condition: Condvar::new(),
            consumer_condition: Condvar::new(),
            blocked: None,
        }
    }

    pub fn put_message(&self, item: TcpMessageItem) {
        {
            let mut lock = self.state.lock().unwrap();
            while lock.entries.len() >= self.max_entries && !lock.stopped {
                if let Some(callback) = &self.blocked {
                    callback();
                }
                lock = self.producer_condition.wait(lock).unwrap();
            }
            lock.entries.push_back(item);
        }
        self.consumer_condition.notify_one();
    }

    pub fn get_message(&self) -> TcpMessageItem {
        let result = {
            let mut lock = self.state.lock().unwrap();
            while lock.entries.is_empty() && !lock.stopped {
                lock = self.consumer_condition.wait(lock).unwrap();
            }
            if !lock.entries.is_empty() {
                lock.entries.pop_front().unwrap()
            } else {
                TcpMessageItem::new()
            }
        };
        self.producer_condition.notify_one();
        result
    }

    pub fn size(&self) -> usize {
        self.state.lock().unwrap().entries.len()
    }

    /// Stop container and notify waiting threads
    pub fn stop(&self) {
        {
            let mut lock = self.state.lock().unwrap();
            lock.stopped = true;
        }
        self.consumer_condition.notify_all();
        self.producer_condition.notify_all();
    }
}

const MAX_ENTRIES_PER_CONNECTION: usize = 16;

#[cfg(test)]
mod tests {
    use std::thread::spawn;

    use super::*;

    #[test]
    fn put_and_get_one_message() {
        let manager = TcpMessageManager::new(1);
        let mut item = TcpMessageItem::new();
        item.node_id = Account::from_bytes([1; 32]);
        assert_eq!(manager.size(), 0);
        manager.put_message(item.clone());
        assert_eq!(manager.size(), 1);
        assert_eq!(manager.get_message().node_id, item.node_id);
        assert_eq!(manager.size(), 0);
    }

    #[test]
    fn block_when_max_entries_reached() {
        let mut manager = TcpMessageManager::new(1);
        let blocked_notification = Arc::new((Mutex::new(false), Condvar::new()));
        let blocked_notification2 = blocked_notification.clone();
        manager.blocked = Some(Box::new(move || {
            let (mutex, condvar) = blocked_notification2.as_ref();
            let mut lock = mutex.lock().unwrap();
            *lock = true;
            condvar.notify_one();
        }));
        let manager = Arc::new(manager);

        let mut item = TcpMessageItem::new();
        item.node_id = Account::from_bytes([1; 32]);

        // Fill the queue
        for _ in 0..manager.max_entries {
            manager.put_message(item.clone());
        }

        assert_eq!(manager.size(), manager.max_entries);

        // This task will wait until a message is consumed
        let manager_clone = manager.clone();
        let handle = spawn(move || {
            manager_clone.put_message(item);
        });

        let (mutex, condvar) = blocked_notification.as_ref();
        let mut lock = mutex.lock().unwrap();
        while !*lock {
            lock = condvar.wait(lock).unwrap();
        }

        assert_eq!(manager.size(), manager.max_entries);
        manager.get_message();
        assert!(handle.join().is_ok());
        assert_eq!(manager.size(), manager.max_entries);
    }

    #[test]
    fn bulk_test() {
        let manager = Arc::new(TcpMessageManager::new(2));
        let message_count = 50;

        let mut item = TcpMessageItem::new();
        item.node_id = Account::from_bytes([1; 32]);

        let consumers: Vec<_> = (0..4)
            .map(|_| {
                let item = item.clone();
                let manager = Arc::clone(&manager);
                spawn(move || {
                    for _ in 0..message_count {
                        let msg = manager.get_message();
                        assert_eq!(msg.node_id, item.node_id);
                    }
                })
            })
            .collect();

        let producers: Vec<_> = (0..4)
            .map(|_| {
                let item = item.clone();
                let manager = Arc::clone(&manager);
                spawn(move || {
                    for _ in 0..message_count {
                        manager.put_message(item.clone());
                    }
                })
            })
            .collect();

        for handle in consumers {
            handle.join().unwrap();
        }

        for handle in producers {
            handle.join().unwrap();
        }
    }
}
