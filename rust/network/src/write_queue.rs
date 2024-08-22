use crate::TrafficType;
use std::sync::Arc;
use tokio::sync::mpsc::{self};

pub struct WriteQueue {
    generic_queue: mpsc::Sender<Entry>,
    bootstrap_queue: mpsc::Sender<Entry>,
}

impl WriteQueue {
    pub fn new(max_size: usize) -> (Self, WriteQueueReceiver) {
        let (generic_tx, generic_rx) = mpsc::channel(max_size * 2);
        let (bootstrap_tx, bootstrap_rx) = mpsc::channel(max_size * 2);
        let receiver = WriteQueueReceiver::new(generic_rx, bootstrap_rx);
        (
            Self {
                generic_queue: generic_tx,
                bootstrap_queue: bootstrap_tx,
            },
            receiver,
        )
    }

    pub async fn insert(
        &self,
        buffer: Arc<Vec<u8>>,
        traffic_type: TrafficType,
    ) -> anyhow::Result<()> {
        let entry = Entry { buffer };
        self.queue_for(traffic_type)
            .send(entry)
            .await
            .map_err(|_| anyhow!("queue closed"))
    }

    /// returns: inserted | write_error
    pub fn try_insert(&self, buffer: Arc<Vec<u8>>, traffic_type: TrafficType) -> (bool, bool) {
        let entry = Entry { buffer };
        match self.queue_for(traffic_type).try_send(entry) {
            Ok(()) => (true, false),
            Err(mpsc::error::TrySendError::Full(_)) => (false, false),
            Err(mpsc::error::TrySendError::Closed(_)) => (false, true),
        }
    }

    pub fn capacity(&self, traffic_type: TrafficType) -> usize {
        self.queue_for(traffic_type).capacity()
    }

    fn queue_for(&self, traffic_type: TrafficType) -> &mpsc::Sender<Entry> {
        match traffic_type {
            TrafficType::Generic => &self.generic_queue,
            TrafficType::Bootstrap => &self.bootstrap_queue,
        }
    }
}

pub struct WriteQueueReceiver {
    generic: mpsc::Receiver<Entry>,
    bootstrap: mpsc::Receiver<Entry>,
}

impl WriteQueueReceiver {
    fn new(generic: mpsc::Receiver<Entry>, bootstrap: mpsc::Receiver<Entry>) -> Self {
        Self { generic, bootstrap }
    }

    pub async fn pop(&mut self) -> Option<(Entry, TrafficType)> {
        // always prefer generic queue!
        if let Ok(result) = self.generic.try_recv() {
            return Some((result, TrafficType::Generic));
        }

        tokio::select! {
            v = self.generic.recv() => v.map(|i| (i, TrafficType::Generic)),
            v = self.bootstrap.recv() => v.map(|i| (i, TrafficType::Bootstrap)),
        }
    }
}

pub struct Entry {
    pub buffer: Arc<Vec<u8>>,
}
